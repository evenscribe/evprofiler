use crate::{
    profile::{
        self,
        schema::{
            COLUMN_NAME, COLUMN_PERIOD_TYPE, COLUMN_PERIOD_UNIT, COLUMN_SAMPLE_TYPE,
            COLUMN_SAMPLE_UNIT, COLUMN_STACKTRACE, COLUMN_TIMESTAMP, COLUMN_VALUE,
        },
        utils,
    },
    schema_builder::{self, symbolized_record_schema},
    symbolizer::Symbolizer,
};
use datafusion::{
    arrow::{
        array::{
            record_batch, Array, ArrayBuilder, AsArray, BinaryDictionaryBuilder,
            GenericListBuilder, Int64Builder, ListBuilder, NullArray, RecordBatch, StructBuilder,
            UInt64Builder,
        },
        datatypes::{DataType, Field, Fields, Int32Type, Schema, SchemaBuilder},
    },
    catalog::TableProvider,
    datasource::{
        file_format::parquet::ParquetFormat,
        listing::{ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl},
    },
    functions_aggregate::sum::sum,
    parquet::data_type::AsBytes,
    prelude::*,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

struct CachedProvider {
    provider: Arc<dyn TableProvider>,
    created_at: Instant,
}

impl CachedProvider {
    fn new(provider: Arc<dyn TableProvider>) -> Self {
        Self {
            provider,
            created_at: Instant::now(),
        }
    }
}

pub struct DataAccessLayer {
    path_prefix: String,
    max_cache_stale_duration: Duration,
    config: ListingTableConfig,
    cached_provider: Mutex<CachedProvider>,
    symbolizer: Arc<Symbolizer>,
}

#[derive(Debug)]
struct QueryParts {
    meta: profile::Meta,
    label_filters: Vec<Expr>,
}

impl DataAccessLayer {
    pub async fn try_new(path: &str, cache_stale_duration: u64) -> anyhow::Result<Self> {
        let ctx = SessionContext::new();
        let session_state = ctx.state();
        let table_path = ListingTableUrl::parse(path)?;

        let file_format = ParquetFormat::new();
        let listing_options =
            ListingOptions::new(Arc::new(file_format)).with_file_extension(".parquet");

        let resolved_schema = listing_options
            .infer_schema(&session_state, &table_path)
            .await?;

        let config = ListingTableConfig::new(table_path)
            .with_listing_options(listing_options)
            .with_schema(resolved_schema);

        let provider = Arc::new(ListingTable::try_new(config.clone())?);

        Ok(Self {
            max_cache_stale_duration: Duration::new(cache_stale_duration, 0),
            path_prefix: path.to_string(),
            cached_provider: Mutex::new(CachedProvider::new(provider)),
            config,
        })
    }

    async fn get_provider(&self) -> anyhow::Result<Arc<dyn TableProvider>> {
        let mut cp = self.cached_provider.lock().unwrap();
        if cp.created_at.elapsed() < self.max_cache_stale_duration {
            return Ok(Arc::clone(&cp.provider));
        }

        let cp_ = self.create_cached_provider()?;
        let p = Arc::clone(&cp.provider);
        *cp = cp_;

        return Ok(p);
    }

    fn create_cached_provider(&self) -> anyhow::Result<CachedProvider> {
        let p = ListingTable::try_new(self.config.clone())?;
        Ok(CachedProvider::new(Arc::new(p)))
    }

    pub async fn select_single(&self, qs: &str, time: i64) -> anyhow::Result<profile::Profile> {
        let (records, value_col, meta) = self.find_single(qs, time).await?;

        let symbolized_records: Vec<RecordBatch> =
            self.symbolize_records(records, value_col, &meta).await?;

        let mut total_rows = 0;
        for record in symbolized_records.iter() {
            total_rows += record.num_rows()
        }

        if total_rows == 0 {
            anyhow::bail!("Could not find profile at requested time and selectors")
        }

        Ok(profile::Profile {
            meta,
            samples: symbolized_records,
        })
    }

    async fn find_single(
        &self,
        qs: &str,
        time: i64,
    ) -> anyhow::Result<(Vec<RecordBatch>, &str, profile::Meta)> {
        let (mut meta, mut filter_expr) = qs_to_meta_and_filter_expr(qs)?;
        filter_expr.push(col(COLUMN_TIMESTAMP).eq(lit(time)));

        let filter_expr = filter_expr
            .into_iter()
            .reduce(|acc, pred| and(acc, pred))
            .unwrap();

        let ctx = SessionContext::new();
        let value_column = "sum(value)";
        let aggr_expr = vec![sum(col(COLUMN_VALUE)).alias(value_column)];
        let group_expr = vec![col(COLUMN_STACKTRACE)];
        let df = ctx.read_table(self.get_provider().await?)?;
        let df = df.filter(filter_expr)?;
        let df = df.aggregate(group_expr, aggr_expr)?;
        let record = df.collect().await?;

        meta.timestamp = time;

        Ok((record, value_column, meta))
    }

    async fn symbolize_records(
        &self,
        records: Vec<RecordBatch>,
        value_col: &str,
        _: &profile::Meta,
    ) -> anyhow::Result<Vec<RecordBatch>> {
        let mut res = Vec::with_capacity(records.len());

        for record in records.iter() {
            let stacktrace_col = Arc::clone(match record.column_by_name(COLUMN_STACKTRACE) {
                Some(sc) => sc,
                None => anyhow::bail!("Missing column: {}", COLUMN_STACKTRACE),
            });
            let value_column = Arc::clone(match record.column_by_name(value_col) {
                Some(sc) => sc,
                None => anyhow::bail!("Missing column: {}", value_col),
            });
            let values_per_second = Arc::new(NullArray::new(value_col.len()));
            let locations_record = self.resolve_stacks(stacktrace_col).await?;

            let records = vec![
                Arc::clone(locations_record.column(0)),
                value_column,
                values_per_second,
            ];

            let record_batch = RecordBatch::try_new(Arc::new(symbolized_record_schema()), records)?;
            res.push(record_batch);
        }

        Ok(res)
    }

    async fn resolve_stacks(&self, stacktrace_col: Arc<dyn Array>) -> anyhow::Result<RecordBatch> {
        let stacktrace_col = match stacktrace_col.as_fixed_size_binary_opt() {
            Some(sc) => sc,
            None => anyhow::bail!("stacktrace column couldnot be downcasted to binary array."),
        };

        let symbolized_locations =
            utils::symbolize_locations(stacktrace_col, Arc::clone(&self.symbolizer)).await?;

        let mut locations_list = locations_array_builder();
        for (indx, stacktrace) in stacktrace_col.iter().enumerate() {
            if stacktrace.is_none() {
                locations_list.append_null();
                continue;
            }
            locations_list.append(true);

            let locations: &mut StructBuilder = locations_list.values();
            locations.append(true);

            let addresses = locations.field_builder::<UInt64Builder>(0).unwrap();
            if let Some(symbolized_location) = &symbolized_locations[indx] {
                addresses.append_value(symbolized_location.address);

                if let Some(mapping) = &symbolized_location.mapping {
                    let mapping_build_id = locations
                        .field_builder::<BinaryDictionaryBuilder<Int32Type>>(5)
                        .unwrap();
                    if !mapping.build_id.is_empty() {
                        mapping_build_id.append_value(mapping.build_id.as_bytes());
                    } else {
                        mapping_build_id.append_value("".as_bytes());
                    }

                    let mapping_file = locations
                        .field_builder::<BinaryDictionaryBuilder<Int32Type>>(4)
                        .unwrap();
                    if !mapping.file.is_empty() {
                        mapping_file.append_value(mapping.file.as_bytes());
                    } else {
                        mapping_file.append_value("".as_bytes());
                    }

                    let mapping_start = locations.field_builder::<UInt64Builder>(1).unwrap();
                    mapping_start.append_value(mapping.start);

                    let mapping_limit = locations.field_builder::<UInt64Builder>(2).unwrap();
                    mapping_limit.append_value(mapping.limit);

                    let mapping_offset = locations.field_builder::<UInt64Builder>(3).unwrap();
                    mapping_offset.append_value(mapping.offset);
                } else {
                    let mapping_build_id = locations
                        .field_builder::<BinaryDictionaryBuilder<Int32Type>>(5)
                        .unwrap();
                    mapping_build_id.append_value("".as_bytes());

                    let mapping_file = locations
                        .field_builder::<BinaryDictionaryBuilder<Int32Type>>(4)
                        .unwrap();
                    mapping_file.append_value("".as_bytes());

                    let mapping_start = locations.field_builder::<UInt64Builder>(1).unwrap();
                    mapping_start.append_value(0);

                    let mapping_limit = locations.field_builder::<UInt64Builder>(2).unwrap();
                    mapping_limit.append_value(0);

                    let mapping_offset = locations.field_builder::<UInt64Builder>(3).unwrap();
                    mapping_offset.append_value(0);
                }

                let lines = locations
                    .field_builder::<ListBuilder<Box<dyn ArrayBuilder>>>(6)
                    .unwrap();
                if symbolized_location.lines.len() > 0 {
                    lines.append(true);
                    for ln in symbolized_location.lines.iter() {
                        let line = lines
                            .values()
                            .as_any_mut()
                            .downcast_mut::<StructBuilder>()
                            .unwrap();
                        line.append(true);

                        let line_number = line.field_builder::<Int64Builder>(0).unwrap();
                        line_number.append_value(ln.line);

                        if let Some(func) = &ln.function {
                            let function_name = line
                                .field_builder::<BinaryDictionaryBuilder<Int32Type>>(1)
                                .unwrap();
                            if !func.name.is_empty() {
                                function_name.append_value(func.name.as_bytes());
                            } else {
                                function_name.append_value("".as_bytes());
                            }

                            let function_system_name = line
                                .field_builder::<BinaryDictionaryBuilder<Int32Type>>(2)
                                .unwrap();
                            if !func.system_name.is_empty() {
                                function_system_name.append_value(func.system_name.as_bytes());
                            } else {
                                function_system_name.append_value("".as_bytes());
                            }

                            let function_filename = line
                                .field_builder::<BinaryDictionaryBuilder<Int32Type>>(3)
                                .unwrap();
                            if !func.filename.is_empty() {
                                function_filename.append_value(func.filename.as_bytes());
                            } else {
                                function_filename.append_value("".as_bytes());
                            }

                            let function_start_line =
                                line.field_builder::<Int64Builder>(4).unwrap();
                            function_start_line.append_value(func.start_line);
                        }
                    }
                } else {
                    lines.append(false);
                }
            } else {
                addresses.append_value(0);

                let lines = locations
                    .field_builder::<ListBuilder<Box<dyn ArrayBuilder>>>(6)
                    .unwrap();
                lines.append_null();
            }
        }

        let locations_array = locations_list.finish();
        Ok(RecordBatch::try_new(
            Arc::new(schema_builder::locations_arrow_schema()),
            vec![Arc::new(locations_array)],
        )?)
    }
}

fn locations_array_builder() -> GenericListBuilder<i32, StructBuilder> {
    ListBuilder::new(StructBuilder::from_fields(
        vec![
            Field::new("address", DataType::UInt64, true),
            Field::new("mapping_start", DataType::UInt64, true),
            Field::new("mapping_limit", DataType::UInt64, true),
            Field::new("mapping_offset", DataType::UInt64, true),
            Field::new(
                "mapping_file",
                DataType::Dictionary(Box::new(DataType::UInt32), Box::new(DataType::Binary)),
                true,
            ),
            Field::new(
                "mapping_build_id",
                DataType::Dictionary(Box::new(DataType::UInt32), Box::new(DataType::Binary)),
                true,
            ),
            Field::new(
                "lines",
                DataType::List(Arc::new(Field::new_list_field(
                    DataType::Struct(Fields::from(vec![
                        Field::new("line", DataType::Int64, true),
                        Field::new(
                            "function_name",
                            DataType::Dictionary(
                                Box::new(DataType::UInt32),
                                Box::new(DataType::Binary),
                            ),
                            true,
                        ),
                        Field::new(
                            "function_system_name",
                            DataType::Dictionary(
                                Box::new(DataType::UInt32),
                                Box::new(DataType::Binary),
                            ),
                            true,
                        ),
                        Field::new(
                            "function_filename",
                            DataType::Dictionary(
                                Box::new(DataType::UInt32),
                                Box::new(DataType::Binary),
                            ),
                            true,
                        ),
                        Field::new("function_start_line", DataType::Int64, true),
                    ])),
                    true,
                ))),
                true,
            ),
        ],
        0,
    ))
}

fn qs_to_meta_and_filter_expr(qs: &str) -> anyhow::Result<(profile::Meta, Vec<Expr>)> {
    let parsed_query: Vec<&str> = qs.trim().split("|").collect();
    if parsed_query.len() != 2 {
        anyhow::bail!("Expected 2 part query but received {}. Make sure it is in this format: <labels_name>=xx,<labels_name>=yy,...|<name>:<sample-type>:<sample-unit>:<period-type>:<period-unit>", parsed_query.len());
    }

    let labels_matcher: Vec<&str> = parsed_query[0].split(",").map(|lm| lm.trim()).collect();
    let mut filter_expressions: Vec<Expr> = Vec::with_capacity(labels_matcher.len());
    for lm in labels_matcher {
        let parsed_lm: Vec<&str> = lm.trim().split("=").collect();
        if parsed_lm.len() != 2 {
            anyhow::bail!("Expected 2 part label matcher but received {}. Make sure it is in this format: <label_name>=xx", parsed_lm.len());
        }
        let col_name = format!(r#""labels.{}""#, parsed_lm[0]);

        filter_expressions.push(col(col_name).eq(lit(parsed_lm[1])));
    }

    let meta_fields: Vec<&str> = parsed_query[1].trim().split(":").collect();
    if meta_fields.len() != 5 {
        anyhow::bail!("Expected 5 meta fields but received {}. Make sure it is in this format: <name>:<sample-type>:<sample-unit>:<period-type>:<period-unit> ", meta_fields.len());
    }

    let meta = profile::Meta {
        name: meta_fields[0].into(),
        sample_type: profile::ValueType {
            type_: meta_fields[1].into(),
            unit: meta_fields[2].into(),
        },
        period_type: profile::ValueType {
            type_: meta_fields[3].into(),
            unit: meta_fields[4].into(),
        },
        timestamp: 0,
        duration: 0,
        period: 0,
    };

    filter_expressions.push(col(COLUMN_NAME).eq(lit(meta_fields[0])));
    filter_expressions.push(col(COLUMN_SAMPLE_TYPE).eq(lit(meta_fields[1])));
    filter_expressions.push(col(COLUMN_SAMPLE_UNIT).eq(lit(meta_fields[2])));
    filter_expressions.push(col(COLUMN_PERIOD_TYPE).eq(lit(meta_fields[3])));
    filter_expressions.push(col(COLUMN_PERIOD_UNIT).eq(lit(meta_fields[4])));

    Ok((meta, filter_expressions))
}

///// qs is expected to be be of the form <labels_name>=xx,<labels_name>=yy,...|<name>:<sample-type>:<sample-unit>:<period-type>:<period-unit>
/////
///// # Errors
/////
///// This function will return an error if the qs doesn't satisfy the format.
//fn parse_query(qs: &str) -> anyhow::Result<QueryParts> {
//    let parsed_query: Vec<&str> = qs.trim().split("|").collect();
//    if parsed_query.len() != 2 {
//        anyhow::bail!("Expected 2 part query but received {}. Make sure it is in this format: <labels_name>=xx,<labels_name>=yy,...|<name>:<sample-type>:<sample-unit>:<period-type>:<period-unit>", parsed_query.len());
//    }
//
//    let labels_matcher: Vec<&str> = parsed_query[0].split(",").map(|lm| lm.trim()).collect();
//    let mut matchers: Vec<Expr> = Vec::with_capacity(labels_matcher.len());
//    for lm in labels_matcher {
//        let parsed_lm: Vec<&str> = lm.trim().split("=").collect();
//        if parsed_lm.len() != 2 {
//            anyhow::bail!("Expected 2 part label matcher but received {}. Make sure it is in this format: <label_name>=xx", parsed_lm.len());
//        }
//        let col_name = format!(r#""labels.{}""#, parsed_lm[0]);
//
//        matchers.push(col(col_name).eq(lit(parsed_lm[1])));
//        //matchers.push(col(r#""labels.compiler""#).eq(lit(1)));
//    }
//    //matchers.push(col(r#""labels.compiler""#).eq(lit(1)));
//
//    let meta_fields: Vec<&str> = parsed_query[1].trim().split(":").collect();
//    if meta_fields.len() != 5 {
//        anyhow::bail!("Expected 5 meta fields but received {}. Make sure it is in this format: <name>:<sample-type>:<sample-unit>:<period-type>:<period-unit> ", meta_fields.len());
//    }
//
//    let meta = profile::Meta {
//        name: meta_fields[0].into(),
//        sample_type: profile::ValueType {
//            type_: meta_fields[1].into(),
//            unit: meta_fields[2].into(),
//        },
//        period_type: profile::ValueType {
//            type_: meta_fields[3].into(),
//            unit: meta_fields[4].into(),
//        },
//        timestamp: 0,
//        duration: 0,
//        period: 0,
//    };
//
//    Ok(QueryParts {
//        meta,
//        label_filters: matchers,
//    })
//}

#[cfg(test)]
mod tests {

    use crate::profile;

    use super::*;
    use datafusion::{
        arrow::{
            array::{Array, NullArray, RecordBatch},
            datatypes::Field,
        },
        functions_aggregate::sum::sum,
    };

    #[tokio::test]
    async fn test_provider_works() {
        let dal = DataAccessLayer::try_new("evprofiler-data", 5000)
            .await
            .unwrap();

        let x = "ss";

        let ctx = SessionContext::new();

        let aggr_expr = vec![sum(col("value")).alias("sum(value)")];
        let df = ctx.read_table(dal.get_provider().await.unwrap()).unwrap();
        let df = df.filter(col("duration").eq(lit("9973060593"))).unwrap();
        let df = df
            .aggregate(
                vec![
                    col("stacktrace"),
                    col("duration"),
                    col(r#""labels.compiler""#),
                    col(r#""labels.executable""#),
                ],
                aggr_expr,
            )
            .unwrap();

        //df.show().await.unwrap();
        let meta = profile::Meta {
            name: "parca_agent_cpu".into(),
            period_type: profile::ValueType {
                type_: "cpu".into(),
                unit: "nanoseconds".into(),
            },
            sample_type: profile::ValueType {
                type_: "samples".into(),
                unit: "count".into(),
            },

            timestamp: 1734496663875,
            duration: 9973060593,
            period: 52631578,
        };

        let record = df.collect().await.unwrap();
        assert_ne!(record.len(), 0);
        let r = record.first().unwrap();
        println!("{:?}", r.schema());

        //sum(?table?.value)
    }

    //#[test]
    //fn test_valid_query() -> anyhow::Result<()> {
    //    let query = "executable=/path/to/exec|metric:count:requests:time:seconds";
    //    let result = parse_query(query)?;
    //
    //    assert_eq!(result.meta.name, "metric");
    //    assert_eq!(result.meta.sample_type.type_, "count");
    //    assert_eq!(result.meta.sample_type.unit, "requests");
    //    assert_eq!(result.meta.period_type.type_, "time");
    //    assert_eq!(result.meta.period_type.unit, "seconds");
    //    assert_eq!(result.label_filters.len(), 1);
    //
    //    assert_eq!(parse_query("  labels.name=value1  ,  labels.type=value2  |  metric:count:requests:time:seconds  ").is_ok(), true);
    //    Ok(())
    //}
    //
    //#[test]
    //fn test_invalid_query() -> anyhow::Result<()> {
    //    assert_ne!(parse_query("hh").is_ok(), true);
    //    assert_ne!(
    //        parse_query("labels.name=value1,labels.type=value2").is_ok(),
    //        true
    //    );
    //    assert_ne!(
    //        parse_query("labels.name=value1|metric:count:requests|time:seconds").is_ok(),
    //        true
    //    );
    //    assert_ne!(
    //        parse_query("labels.name=value1=extra|metric:count:requests:time:seconds").is_ok(),
    //        true
    //    );
    //    assert_ne!(
    //        parse_query("labels.name=value1|metric:count:requests:time").is_ok(),
    //        true
    //    );
    //    Ok(())
    //}
}
