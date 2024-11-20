use crate::symbolizer::ElfFile;
use elf::{
    abi::{PF_X, PT_LOAD},
    endian::AnyEndian,
    segment::ProgramHeader,
};
use tonic::Status;

#[derive(Debug, Clone)]
pub struct ProgHeader {
    pub(crate) offset: u64,
    pub(crate) vaddr: u64,
    pub(crate) memsz: u64,
}

pub struct Mapping {
    pub start: u64,
    pub end: u64,
    pub offset: u64,
    pub file: String,
}

pub struct ExecutableInfo {
    pub(crate) elf_type: u16,
    text_prog_hdr_indx: i16,
    prog_headers: Vec<ProgHeader>,
}

impl ExecutableInfo {
    /// FindProgramHeader returns the program segment that matches the current
    /// mapping and the given address, or an error if it cannot find a unique program
    /// header.
    pub(crate) fn find_program_header(
        &self,
        m: &Mapping,
        addr: u64,
    ) -> Result<Option<ProgHeader>, Status> {
        // For user space executables, we try to find the actual program segment that
        // is associated with the given mapping. Skip this search if limit <= start.
        if m.start >= m.end || (m.end) > (1 << 63) {
            return Err(Status::invalid_argument("Invalid mapping"));
        }

        // Some ELF files don't contain any loadable program segments, e.g. .ko
        // kernel modules. It's not an error to have no header in such cases.
        if self.prog_headers.is_empty() {
            return Ok(None);
        }

        let headers: Vec<ProgHeader> = self.program_headers_for_mapping(m.offset, m.end - m.start);
        if headers.is_empty() {
            return Err(Status::internal("No program header matches mapping info"));
        }

        if headers.len() == 1 {
            return Ok(Some(headers.get(0).unwrap().clone()));
        }

        return header_for_file_offset(headers, addr - m.start + m.offset);
    }
    /// ProgramHeadersForMapping returns the program segment headers that overlap
    /// the runtime mapping with file offset mapOff and memory size mapSz. We skip
    /// over segments zero file size because their file offset values are unreliable.
    /// Even if overlapping, a segment is not selected if its aligned file offset is
    /// greater than the mapping file offset, or if the mapping includes the last
    /// page of the segment, but not the full segment and the mapping includes
    /// additional pages after the segment end.
    /// The function returns a vector of headers in the input
    /// slice, which are valid only while phdrs is not modified or discarded.
    ///
    fn program_headers_for_mapping(&self, map_off: u64, map_sz: u64) -> Vec<ProgHeader> {
        // pageSize defines the virtual memory page size used by the loader. This
        // value is dependent on the memory management unit of the CPU. The page
        // size is 4KB virtually on all the architectures that we care about, so we
        // define this metric as a constant. If we encounter architectures where
        // page size is not 4KB, we must try to guess the page size on the system
        // where the profile was collected, possibly using the architecture
        // specified in the ELF file header.
        let page_sz: u64 = 4096;
        let page_mask = page_sz - 1;

        let map_limit = map_off + map_sz;
        let mut headers: Vec<ProgHeader> = vec![];

        for header in self.prog_headers.iter() {
            let seg_limit = header.offset + header.memsz;

            // The segment must overlap the mapping.
            if map_off < seg_limit && header.offset < map_limit {
                // If the mapping offset is strictly less than the page aligned segment
                // offset, then this mapping comes from a different segment, fixes
                // b/179920361.
                let mut aligned_offset: u64 = 0;

                if header.offset > (header.vaddr & page_mask) {
                    aligned_offset = header.offset - (header.vaddr & page_mask);
                }

                if map_off < aligned_offset {
                    continue;
                }
                // If the mapping starts in the middle of the segment, it covers less than
                // one page of the segment, and it extends at least one page past the
                // segment, then this mapping comes from a different segment.
                //
                if map_off > header.offset
                    && (seg_limit < map_off + page_sz)
                    && (map_limit >= seg_limit + page_sz)
                {
                    continue;
                }

                headers.push(header.clone());
            }
        }

        headers
    }
}

/// HeaderForFileOffset attempts to identify a unique program header that
/// includes the given file offset. It returns an error if it cannot identify a
/// unique header.
fn header_for_file_offset(
    headers: Vec<ProgHeader>,
    file_offset: u64,
) -> Result<Option<ProgHeader>, Status> {
    let mut found: Option<ProgHeader> = None;
    for header in headers.iter() {
        if header.offset <= file_offset && file_offset < header.offset + header.memsz {
            if found.is_some() {
                // Assuming no other bugs, this can only happen if we have two or
                // more small program segments that fit on the same page, and a
                // segment other than the last one includes uninitialized data, or
                // if the debug binary used for symbolization is stripped of some
                // sections, so segment file sizes are smaller than memory sizes.
                return Err(Status::internal(format!("found second program header {:?} that matches file offset {:?}, first program header is {:?}. Is this a stripped binary, or does the first program segment contain uninitialized data?", header, file_offset, found.unwrap())));
            }
            found = Some(header.clone());
        }
    }

    match found {
        Some(header) => return Ok(Some(header)),
        None => return Err(Status::internal("No program header matches file offset")),
    }
}

impl TryFrom<&ElfFile<AnyEndian>> for ExecutableInfo {
    type Error = Status;
    fn try_from(e: &ElfFile<AnyEndian>) -> Result<Self, Self::Error> {
        let prog_headers = match e.elf.segments() {
            Some(segments) => segments.iter().collect(),
            None => vec![],
        };

        let text_prog_hdr_indx = find_text_prog_hdr(e, &prog_headers);

        let mut prof_headers_ = vec![];
        prog_headers.iter().enumerate().for_each(|(_, header)| {
            prof_headers_.push(ProgHeader {
                offset: header.p_offset,
                vaddr: header.p_vaddr,
                memsz: header.p_memsz,
            });
        });

        Ok(ExecutableInfo {
            elf_type: e.elf.ehdr.e_type,
            text_prog_hdr_indx,
            prog_headers: prof_headers_,
        })
    }
}

pub fn find_text_prog_hdr(e: &ElfFile<AnyEndian>, prog_headers: &[ProgramHeader]) -> i16 {
    let (shdrs_opt, strtab_opt) = e
        .elf
        .section_headers_with_strtab()
        .expect("shdrs offsets should be valid");
    let (shdrs, strtab) = (
        shdrs_opt.expect("Should have shdrs"),
        strtab_opt.expect("Should have strtab"),
    );

    for shdr in shdrs.iter() {
        if let Ok(name) = strtab.get(shdr.sh_name as usize) {
            if name == ".text" {
                for (idx, header) in prog_headers.iter().enumerate() {
                    if header.p_type == PT_LOAD
                        && header.p_flags & PF_X != 0
                        && shdr.sh_addr >= header.p_vaddr
                        && shdr.sh_addr < header.p_vaddr + header.p_memsz
                    {
                        return idx as i16;
                    }
                }
            }
        }
    }

    return -1;
}
