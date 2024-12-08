use object::{elf::PF_X, File, Object, ObjectKind, ObjectSection, ObjectSegment, SegmentFlags};
use tonic::Status;

#[derive(Debug, Clone)]
pub struct ProgHeader {
    pub(crate) offset: u64,
    pub(crate) vaddr: u64,
    pub(crate) memsz: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Mapping {
    pub start: u64,
    pub end: u64,
    pub offset: u64,
    pub file: String,
}

pub struct ExecutableInfo {
    pub(crate) elf_type: ObjectKind,
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
            return Ok(Some(headers.first().unwrap().clone()));
        }

        header_for_file_offset(headers, addr - m.start + m.offset)
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
        Some(header) => Ok(Some(header)),
        None => Err(Status::internal("No program header matches file offset")),
    }
}

impl TryFrom<&File<'_>> for ExecutableInfo {
    type Error = Status;
    fn try_from(e: &File<'_>) -> Result<Self, Self::Error> {
        let idx = find_text_prog_hdr(e);

        let mut prog_headers: Vec<ProgHeader> = vec![];

        let segments = e.segments();
        for segment in segments {
            prog_headers.push(ProgHeader {
                offset: segment.file_range().0,
                vaddr: segment.address(),
                memsz: segment.size(),
            });
        }

        Ok(ExecutableInfo {
            elf_type: e.kind(),
            text_prog_hdr_indx: idx,
            prog_headers,
        })
    }
}

/// find_text_prog_hdr inds the program segment header containing the .text
//. section or -1 if the segment cannot be found.
pub fn find_text_prog_hdr(e: &File<'_>) -> i16 {
    let segments = e.segments();
    if let Some(section) = e.section_by_name(".text") {
        for (indx, segment) in segments.enumerate() {
            if let SegmentFlags::Elf { p_flags } = segment.flags() {
                let section_addr = section.address();
                let segment_addr = segment.address();
                let segment_size = segment.size();

                if p_flags & PF_X != 0
                    && section_addr >= segment_addr
                    && section_addr < segment_addr + segment_size
                {
                    return indx as i16;
                }
            }
        }
    }
    -1
}
