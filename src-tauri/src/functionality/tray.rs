/* Un-used
use tauri::image::Image;
use tauri::tray::TrayIcon;
use tauri::AppHandle;
use tray_icon::Icon;


#[tauri::command]
pub fn set_tray_icon(app: AppHandle, event: String) -> tauri::Result<()> {
    println!("Setting tray icon to {}", event.as_str());

    let icon = match event.as_str() {
        "connected" => Some(Image::from_path("../../icons/tray/connected.png")?),
        "disconnected" => Some(Image::from_path("../../icons/icon.png")?),
        "muted" => Some(Image::from_path("../../icons/tray/muted.png")?),
        "deafened" => Some(Image::from_path("../../icons/tray/deafened.png")?),
        "speaking" => Some(Image::from_path("../../icons/tray/speaking.png")?),
        "video" => Some(Image::from_path("../../icons/tray/video.png")?),
        "streaming" => Some(Image::from_path("../../icons/tray/streaming.png")?),
        _ => Some(Image::from_path("../../icons/icon.png")?),
    };

    let tray = app.tray_handle();
    let _ = tray.set_icon(icon);
    let _ = tray.set_icon_as_template(false);

    Ok(())
}
*/
