use {
    log::LevelFilter,
    log4rs::{
        append::{console::ConsoleAppender, file::FileAppender},
        config::{Appender, Config, Root},
    },
    std::env::current_exe,
};

pub fn init_logger() {
    let stdout = ConsoleAppender::builder().build();
    let mut log_file = current_exe().unwrap();
    log_file.pop();
    log_file.push("installer.log");

    let file = FileAppender::builder().build(log_file).unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .build(
            Root::builder()
                .appenders(vec!["file", "stdout"])
                .build(LevelFilter::Debug),
        )
        .unwrap();

    // use handle to change logger configuration at runtime
    let _handle = log4rs::init_config(config).unwrap();
}
