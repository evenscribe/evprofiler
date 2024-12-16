mod bla;

use anyhow::bail;
use arrow2::{
    array::Array,
    chunk::Chunk as Achunk,
    datatypes::{DataType, PhysicalType},
    error::Result,
    io::parquet::{read::ParquetError, write::*},
};
use bla::Bla;
use chrono::Utc;
use object_store::{path::Path, ObjectStore};
use rayon::prelude::*;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::profile::schema;

type Chunk = Achunk<Arc<dyn Array>>;

#[derive(Debug)]
pub struct Ingester {
    chunks: Mutex<Vec<Chunk>>,
    max_size: usize,
    storage: Arc<dyn ObjectStore>,
}

impl Ingester {
    pub fn new(max_size: usize, storage: Arc<dyn ObjectStore>) -> Self {
        Self {
            chunks: vec![].into(),
            max_size,
            storage,
        }
    }

    pub async fn ingest(&self, chunk: Achunk<Arc<dyn Array>>) -> anyhow::Result<()> {
        let mut chunks = self.chunks.lock().unwrap();
        chunks.push(chunk);

        let is_full = chunks.len() >= self.max_size;

        log::info!("Ingested a chunk");

        if is_full {
            let c = chunks.clone();
            chunks.clear();
            let s = Arc::clone(&self.storage);
            tokio::spawn(Self::persist(c, s));
        }

        Ok(())
    }

    async fn persist(chunks: Vec<Chunk>, storage: Arc<dyn ObjectStore>) -> anyhow::Result<()> {
        log::info!("Chunks max_size met. Trying to persist.");
        let schema = schema::create_schema();
        let options = WriteOptions {
            write_statistics: true,
            compression: CompressionOptions::Snappy,
            version: Version::V2,
            data_pagesize_limit: None,
        };

        let encoding_map = |data_type: &DataType| match data_type.to_physical_type() {
            PhysicalType::Dictionary(_) => Encoding::RleDictionary,
            _ => Encoding::Plain,
        };

        let encodings = (&schema.fields)
            .iter()
            .map(|f| transverse(&f.data_type, encoding_map))
            .collect::<Vec<_>>();

        let parquet_schema = to_parquet_schema(&schema)
            .map_err(|e| anyhow::anyhow!("Failed to create Parquet schema: {}", e))?;

        log::info!("Did I come here --just after creating parquet schema--??");

        let row_groups = chunks.iter().map(|chunk| {
            let columns = chunk
                .columns()
                .par_iter()
                .zip(parquet_schema.fields().to_vec())
                .zip(encodings.par_iter())
                .flat_map(move |((array, type_), encoding)| {
                    let encoded_columns =
                        array_to_columns(array, type_, options, encoding).unwrap();
                    encoded_columns
                        .into_iter()
                        .map(|encoded_pages| {
                            let encoded_pages = DynIter::new(encoded_pages.into_iter().map(|x| {
                                x.map_err(|e| ParquetError::InvalidParameter(e.to_string()))
                            }));
                            encoded_pages
                                .map(|page| {
                                    compress(page?, vec![], options.compression)
                                        .map_err(|x| x.into())
                                })
                                .collect::<Result<VecDeque<_>>>()
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Result<Vec<VecDeque<CompressedPage>>>>()?;

            let row_group = DynIter::new(
                columns
                    .into_iter()
                    .map(|column| Result::Ok(DynStreamingIterator::new(Bla::new(column)))),
            );
            Result::Ok(row_group)
        });

        log::info!("row_groups: {:?}", row_groups.len());
        let mut buf: Vec<u8> = vec![];
        let mut writer = match FileWriter::try_new(&mut buf, schema, options) {
            Ok(fw) => fw,
            Err(e) => {
                log::error!("{}", e);
                bail!("{}", e)
            }
        };

        for group in row_groups {
            let group = match group {
                Ok(g) => g,
                Err(e) => {
                    log::error!("{}", e);
                    bail!("{}", e)
                }
            };
            match writer.write(group) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("{}", e);
                }
            };
        }
        let _size = match writer.end(None) {
            Ok(_) => {}
            Err(e) => {
                log::error!("{}", e);
            }
        };

        log::info!("buf::: {:#?}", buf.len());
        let current_date = chrono::Local::now().date_naive();
        let timestamp = Utc::now().timestamp();

        let p = Path::parse(&format!(
            "date={}/{}.parquet",
            current_date.format("%Y-%m-%d").to_string(),
            timestamp
        ))?;

        match storage.put(&p, buf.into()).await {
            Ok(_) => {}
            Err(e) => log::error!("{}", e),
        };
        log::info!("Persisted the parquet chunks to {}", p);
        Ok(())
    }
}
