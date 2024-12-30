use datafusion::arrow::datatypes::{DataType, Field, Fields, Schema, SchemaBuilder};
use std::sync::Arc;

//TODO: Add PprofLocationsArrowSchemaHere
//

pub fn locations_field() -> Field {
    Field::new(
        "locations",
        DataType::List(Arc::new(Field::new(
            "locations_inner",
            DataType::Struct(Fields::from(vec![
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
                    DataType::List(Arc::new(Field::new(
                        "lines_inner",
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
            ])),
            true,
        ))),
        true,
    )
}

pub fn locations_arrow_schema() -> Schema {
    let mut sb = SchemaBuilder::new();
    sb.push(locations_field());
    sb.finish()
}

pub fn symbolized_record_schema() -> Schema {
    let mut sb = SchemaBuilder::new();
    sb.push(locations_field());
    sb.push(Field::new("value", DataType::Int64, true));
    sb.push(Field::new("diff", DataType::Int64, true));
    sb.finish()
}
