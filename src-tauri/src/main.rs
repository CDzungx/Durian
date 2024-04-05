#![cfg_attr(
   all(not(debug_assertions), target_os = "windows"),
   windows_subsystem = "windows"
)]

// use functionality::tray as myTray;
use std::time::Duration;
use tauri::{
   image::Image,
   menu::{MenuBuilder, MenuItemBuilder},
   process::restart,
   tray::{ClickType, TrayIconBuilder},
   App, Manager, WebviewWindowBuilder,
};
//use tauri_plugin_deep_link::DeepLinkExt;

use config::get_config;
use injection::{
   client_mod::{self, load_mods_js},
   injection_runner::{self, PREINJECT},
   local_html, plugin, theme,
};
use processors::{css_preprocess, js_preprocess};
use profiles::init_profiles_folders;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};
use util::{
   helpers,
   logger::log,
   notifications,
   paths::get_webdata_dir,
   process,
   window_helpers::{self, clear_cache_check, set_user_agent},
};

use crate::{
   functionality::window::{after_build, setup_autostart},
   util::logger,
};

mod config;
//mod deep_link;
mod functionality;
mod injection;
mod processors;
mod profiles;
mod release;
mod util;
mod window;

fn create_systray(app: &App) -> Result<(), tauri::Error> {
   let open_btn = MenuItemBuilder::with_id("open".to_string(), "Open").build(app)?;
   let reload_btn = MenuItemBuilder::with_id("reload".to_string(), "Reload").build(app)?;
   let restart_btn = MenuItemBuilder::with_id("restart".to_string(), "Restart").build(app)?;
   let quit_btn = MenuItemBuilder::with_id("quit".to_string(), "Quit").build(app)?;

   let tray_menu = MenuBuilder::new(app)
      .items(&[&open_btn, &reload_btn, &restart_btn, &quit_btn])
      .build()?;

   TrayIconBuilder::new()
      .menu(&tray_menu)
      .title("Dorion")
      .tooltip("Dorion")
      .on_menu_event(move |app, event| {
         let id = event.id().as_ref();
         let window = app.get_webview_window("main").unwrap();
         if id == "quit" {
            println!("quit clicked");
            window.close().unwrap_or_default();
            app.exit(0);
         }
         if id == "open" {
            println!("open clicked");
            window.show().unwrap_or_default();
            window.set_focus().unwrap_or_default();
            window.unminimize().unwrap_or_default();
         }
         if id == "restart" {
            println!("restart clicked");
            restart(&app.env());
         }
         if id == "reload" {
            println!("reload clicked");
            window.eval("window.location.reload();").unwrap_or_default();
         }
      })
      .on_tray_icon_event(|tray, event| {
         let app = tray.app_handle();
         if event.click_type == ClickType::Left {
            // Reopen the window if the tray menu icon is clicked
            match app.get_webview_window("main") {
               Some(win) => {
                  let _ = win.show();
                  let _ = win.set_focus();
                  let _ = win.unminimize();
               }
               None => {}
            }
         }
      })
      .build(app)?
      .set_icon(Some(Image::from_path("icons/icon.png")?))?;

   Ok(())
}

#[tauri::command]
fn should_disable_plugins() -> bool {
   std::env::args().any(|arg| arg == "--disable-plugins")
}

fn main() {
   // Ensure config is created
   config::init();

   // ALso init logging
   logger::init(true);

   let config = get_config();

   std::thread::sleep(Duration::from_millis(200));

   // before anything else, check if the clear_cache file exists
   clear_cache_check();

   // Run the steps to init profiles
   init_profiles_folders();

   // maybe disable hardware acceleration for windows
   if config.disable_hardware_accel.unwrap_or(false) {
      #[cfg(target_os = "windows")]
      {
         let existing_args =
            std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
         std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            format!("{} --disable-gpu", existing_args),
         );
      }
   }

   let context = tauri::generate_context!("tauri.conf.json");
   let dorion_open = process::process_already_exists();
   let client_type = config.client_type.unwrap_or("default".to_string());
   let mut url = String::new();

   if client_type == "default" {
      url += "https://discord.com/app";
   } else {
      url = format!("https://{}.discord.com/app", client_type);
   }

   let parsed = reqwest::Url::parse(&url).unwrap();
   let url_ext = tauri::WebviewUrl::External(parsed);

   // If another process of Dorion is already open, show a dialog
   // in the future I want to actually *reveal* the other runnning process
   // instead of showing a popup, but this is fine for now
   if dorion_open && !config.multi_instance.unwrap_or(false) {
      // Send the dorion://open deep link request
      helpers::open_scheme("dorion://open".to_string()).unwrap_or_default();

      // Exit
      std::process::exit(0);
   }

   // Safemode check
   let safemode = std::env::args().any(|arg| arg == "--safemode");
   log!("Safemode enabled: {}", safemode);

   let client_mods = load_mods_js();

   #[allow(clippy::single_match)]
   tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        //.plugin(tauri_plugin_deep_link::init())
        .invoke_handler(tauri::generate_handler![
          should_disable_plugins,
          functionality::streamer_mode::start_streamer_mode_watcher,
          functionality::window::minimize,
          functionality::window::toggle_maximize,
          functionality::window::close,
          css_preprocess::clear_css_cache,
          css_preprocess::localize_imports,
          js_preprocess::localize_all_js,
          local_html::get_index,
          local_html::get_top_bar,
          local_html::get_extra_css,
          notifications::notif_count,
          notifications::send_notification,
          plugin::load_plugins,
          plugin::get_plugin_list,
          plugin::toggle_plugin,
          plugin::toggle_preload,
          plugin::get_plugin_import_urls,
          client_mod::available_mods,
          client_mod::load_mods_css,
          profiles::get_profile_list,
          profiles::get_current_profile_folder,
          profiles::create_profile,
          profiles::delete_profile,
          release::do_update,
          release::update_check,
          functionality::rpc::get_windows,
          functionality::rpc::get_local_detectables,
          functionality::hotkeys::save_ptt_keys,
          functionality::hotkeys::toggle_ptt,
          injection_runner::get_injection_js,
          injection_runner::is_injected,
          injection_runner::load_injection_js,
          config::read_config_file,
          config::write_config_file,
          config::default_config,
          theme::get_theme,
          theme::get_theme_names,
          theme::theme_from_link,
          helpers::get_platform,
          helpers::open_themes,
          helpers::open_plugins,
          helpers::fetch_image,
          window::blur::available_blurs,
          window::blur::apply_effect,
          // window::blur::remove_effect,
          window_helpers::remove_top_bar,
          window_helpers::set_clear_cache,
          window_helpers::window_zoom_level,
          // myTray::set_tray_icon,
        ])

        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Destroyed { .. } => {
                functionality::cache::maybe_clear_cache();
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // Just hide the window if the config calls for it
                if get_config().sys_tray.unwrap_or(false) {
                    window.hide().unwrap_or_default();
                    api.prevent_close();
                }

                window
                    .app_handle()
                    .save_window_state(StateFlags::all())
                    .unwrap_or_default();
            }
            _ => {}
        })

        .setup(move |app| {
            // Init plugin list
            plugin::get_new_plugins();

            // Load preload plugins into a single string
            let mut preload_str = String::new();

            for script in plugin::load_plugins(Some(true)).values() {
                preload_str += format!("{};", script).as_str();
            }

            // First, grab preload plugins
            let title = format!("Dorion - v{}", app.package_info().version);
            let win = WebviewWindowBuilder::new(app, "main", url_ext)
                .title(title.as_str())
                .initialization_script(
                    format!(
                        "!window.__DORION_INITIALIZED__ && {};{};{}",
                        PREINJECT.as_str(),
                        client_mods,
                        preload_str,
                    ).as_str()
                )
                .auto_resize()
                .disable_drag_drop_handler()
                .data_directory(get_webdata_dir())
                // Prevent flickering by starting hidden, and show later
                .visible(false)
                // .decorations(true)
                // .resizable(true)
                // .maximizable(true)
                // .minimizable(true)
                // .closable(true)
                // .transparent(
                //     config.blur.unwrap_or("none".to_string()) != "none"
                // )
                .center()
                //.fullscreen(false)
                .build()?;

            // Set the user agent to one that enables all normal Discord features
            set_user_agent(&win);

            // Create the system tray
            let _ = create_systray(&app);

            // If safemode is enabled, stop here
            if safemode {
                win.show().unwrap_or_default();
                return Ok(());
            }

            // restore state BEFORE after_build, since that may change the window
            // win.restore_state(StateFlags::all()).unwrap_or_default();

            /* wait rsRPC to update v2 begin the RPC server if needed
            if get_config().rpc_server.unwrap_or(false) {
                let win_cln = win.clone();
                std::thread::spawn(|| {
                    functionality::rpc::start_rpc_server(win_cln);
                });
            }
            */

            after_build(&win);

            setup_autostart(app);

            Ok(())
        })
        .run(context)
        .expect("error while running tauri application");
}
