use simple_tmp_logger::append_log;
use dusa_collection_utils::errors::ErrorArray;

#[derive(Debug)]
pub enum Names {
    AisAggregator,
    AisSystemdMonitor,
    AisGithubMonitor,
    AisApacheMonitor,
    AisInternal,
}

pub fn log(data: String, name: Names) {
    let app_name = format!("{:#?}", name);
    let errors: ErrorArray = ErrorArray::new_container();

    // TODO FIX LOGGING
    if let Err(_e) = append_log(&app_name, &data, errors.clone()).uf_unwrap() {
        // e.display(false);
    }

    drop(errors);
    drop(data);
    drop(app_name);
}

