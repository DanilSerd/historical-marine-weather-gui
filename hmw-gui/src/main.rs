#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod assets;
mod collapsable;
mod collection;
mod consts;
mod controll_bar;
mod data_file_manager;
mod earth_map;
mod loader;
mod types;
mod utils;
mod weather_summary_collection;
mod weather_summary_details;
mod weather_summary_form;
mod weather_summary_list;
mod weather_summary_stats;
mod widgets;
mod windrose;

fn main() {
    app::deamon::run();
}
