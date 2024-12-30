use datafusion::arrow::{
    array::{Array, AsArray, RecordBatch},
    datatypes::Field,
};
use std::sync::Arc;

struct LabelColumn {
    col: Arc<dyn Array>,
    dict: Arc<dyn Array>,
}

pub(crate) struct RecordReader {
    label_fields: Vec<Field>,
    /// Downcastable to GenericListArray<i32>
    pub(crate) locations_col: Arc<dyn Array>,
    /// Downcastable to StructArray
    pub(crate) location_col: Arc<dyn Array>,
    /// Downcastable to UInt64Array
    pub(crate) address_col: Arc<dyn Array>,
    /// Downcastable to UInt64Array
    pub(crate) mapping_start_col: Arc<dyn Array>,
    /// Downcastable to UInt64Array
    pub(crate) mapping_limit_col: Arc<dyn Array>,
    /// Downcastable to UInt64Array
    pub(crate) mapping_offset_col: Arc<dyn Array>,
    /// Downcastable to DictionaryArray<UInt32, Binary>
    pub(crate) mapping_file_col: Arc<dyn Array>,
    /// Downcastable to DictionaryArray<UInt32, Binary>
    pub(crate) mapping_buildid_col: Arc<dyn Array>,
    /// Downcastable to GenericListArray<i32>
    pub(crate) lines_col: Arc<dyn Array>,
    /// Downcastable to StructArray
    pub(crate) line_col: Arc<dyn Array>,
    /// Downcastable to Int64Array
    pub(crate) line_number_col: Arc<dyn Array>,
    /// Downcastable to DictionaryArray<UInt32, Binary>
    pub(crate) line_function_name_col: Arc<dyn Array>,
    /// Downcastable to DictionaryArray<UInt32, Binary>
    pub(crate) line_function_systemname_col: Arc<dyn Array>,
    /// Downcastable to DictionaryArray<UInt32, Binary>
    pub(crate) line_function_filename_col: Arc<dyn Array>,
    /// Downcastable to Int64Array
    pub(crate) line_function_startline_col: Arc<dyn Array>,
    /// Downcastable to Int64Array
    pub(crate) value_col: Arc<dyn Array>,
    /// Downcastable to Int64Array
    pub(crate) diff_col: Arc<dyn Array>,
}

impl RecordReader {
    pub fn new(ar: &RecordBatch) -> Self {
        let locations_col = Arc::clone(ar.column(0));
        let value_col = Arc::clone(ar.column(1));
        let diff_col = Arc::clone(ar.column(2));

        let locations = locations_col.as_list_opt::<i32>().unwrap();

        let location_col = Arc::clone(locations.values());
        let location = location_col.as_struct_opt().unwrap();

        let address_col = Arc::clone(location.column(0));
        let mapping_start_col = Arc::clone(location.column(1));
        let mapping_limit_col = Arc::clone(location.column(2));
        let mapping_offset_col = Arc::clone(location.column(3));
        let mapping_file_col = Arc::clone(location.column(4));
        let mapping_buildid_col = Arc::clone(location.column(5));

        let lines_col = Arc::clone(location.column(6));
        let lines = lines_col.as_list_opt::<i32>().unwrap();

        let line_col = Arc::clone(lines.values());
        let line = lines_col.as_struct_opt().unwrap();

        let line_number_col = Arc::clone(line.column(0));
        let line_function_name_col = Arc::clone(line.column(1));
        let line_function_systemname_col = Arc::clone(line.column(2));
        let line_function_filename_col = Arc::clone(line.column(3));
        let line_function_startline_col = Arc::clone(line.column(4));

        Self {
            label_fields: vec![],
            locations_col,
            value_col,
            diff_col,
            location_col,
            address_col,
            mapping_start_col,
            mapping_limit_col,
            mapping_offset_col,
            mapping_file_col,
            mapping_buildid_col,
            lines_col,
            line_col,
            line_number_col,
            line_function_name_col,
            line_function_systemname_col,
            line_function_filename_col,
            line_function_startline_col,
        }
    }
}
