use arrow2::datatypes::{DataType, Field, IntegerType, Schema};

const COLUMN_DURATION: &str = "duration";
const COLUMN_LABELS: &str = "labels";
const COLUMN_NAME: &str = "name";
const COLUMN_PERIOD: &str = "period";
const COLUMN_PERIOD_TYPE: &str = "period_type";
const COLUMN_PERIOD_UNIT: &str = "period_unit";
const COLUMN_SAMPLE_TYPE: &str = "sample_type";
const COLUMN_SAMPLE_UNIT: &str = "sample_unit";
const COLUMN_STACKTRACE: &str = "stacktrace";
const COLUMN_STACKTRACE_ITEM: &str = "item";
const COLUMN_TIMESTAMP: &str = "timestamp";
const COLUMN_VALUE: &str = "value";

pub fn create_schema(labels: &[String]) -> Schema {
    let mut fields = vec![
        Field::new(COLUMN_DURATION, DataType::Int64, false),
        Field::new(
            COLUMN_NAME,
            DataType::Dictionary(IntegerType::Int32, Box::new(DataType::Utf8), false),
            false,
        ),
        Field::new(COLUMN_PERIOD, DataType::Int64, false),
        Field::new(
            COLUMN_PERIOD_TYPE,
            DataType::Dictionary(IntegerType::Int32, Box::new(DataType::Utf8), false),
            false,
        ),
        Field::new(
            COLUMN_PERIOD_UNIT,
            DataType::Dictionary(IntegerType::Int32, Box::new(DataType::Utf8), false),
            false,
        ),
        Field::new(
            COLUMN_SAMPLE_TYPE,
            DataType::Dictionary(IntegerType::Int32, Box::new(DataType::Utf8), false),
            false,
        ),
        Field::new(
            COLUMN_SAMPLE_UNIT,
            DataType::Dictionary(IntegerType::Int32, Box::new(DataType::Utf8), false),
            false,
        ),
        Field::new(
            COLUMN_STACKTRACE,
            DataType::List(Box::new(Field::new(
                COLUMN_STACKTRACE_ITEM,
                DataType::Binary,
                false,
            ))),
            false,
        ),
        Field::new(COLUMN_TIMESTAMP, DataType::Int64, false),
        Field::new(COLUMN_VALUE, DataType::Int64, false),
    ];

    for label in labels {
        fields.push(Field::new(
            format!("{}.{}", COLUMN_LABELS, label),
            DataType::Dictionary(IntegerType::Int32, Box::new(DataType::Utf8), false),
            true,
        ));
    }

    Schema::from(fields)
}
