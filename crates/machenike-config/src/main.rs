use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Read, Write};
use std::process::{exit, Command};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const CONFIG_DIR: &str = "/etc/machenike";
const CONFIG_FILE: &str = "/etc/machenike/hotkeysd.conf";
const SERVICE: &str = "machenike-hotkeysd.service";

const EV_KEY: u16 = 0x01;

const KEY_1: u16 = 2;
const KEY_2: u16 = 3;
const KEY_3: u16 = 4;
const KEY_4: u16 = 5;
const KEY_5: u16 = 6;
const KEY_6: u16 = 7;
const KEY_7: u16 = 8;
const KEY_8: u16 = 9;
const KEY_9: u16 = 10;
const KEY_0: u16 = 11;

const KEY_MINUS: u16 = 12;
const KEY_EQUAL: u16 = 13;

const KEY_Q: u16 = 16;
const KEY_W: u16 = 17;
const KEY_E: u16 = 18;
const KEY_R: u16 = 19;
const KEY_T: u16 = 20;
const KEY_Y: u16 = 21;
const KEY_U: u16 = 22;
const KEY_I: u16 = 23;
const KEY_O: u16 = 24;
const KEY_P: u16 = 25;

const KEY_A: u16 = 30;
const KEY_S: u16 = 31;
const KEY_D: u16 = 32;
const KEY_F: u16 = 33;
const KEY_G: u16 = 34;
const KEY_H: u16 = 35;
const KEY_J: u16 = 36;
const KEY_K: u16 = 37;
const KEY_L: u16 = 38;

const KEY_Z: u16 = 44;
const KEY_X: u16 = 45;
const KEY_C: u16 = 46;
const KEY_V: u16 = 47;
const KEY_B: u16 = 48;
const KEY_N: u16 = 49;
const KEY_M: u16 = 50;

const KEY_SLASH: u16 = 53;
const KEY_KPASTERISK: u16 = 55;
const KEY_SPACE: u16 = 57;
const KEY_KPMINUS: u16 = 74;
const KEY_KPPLUS: u16 = 78;
const KEY_KPSLASH: u16 = 98;

const KEY_LEFTCTRL: u16 = 29;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_LEFTALT: u16 = 56;
const KEY_LEFTMETA: u16 = 125;

const KEY_RIGHTCTRL: u16 = 97;
const KEY_RIGHTSHIFT: u16 = 54;
const KEY_RIGHTALT: u16 = 100;
const KEY_RIGHTMETA: u16 = 126;

#[derive(Debug, Clone)]
struct Config {
    rgb_hold_delay_ms: u64,
    rgb_step_delay_ms: u64,
    rgb_hue_step: u16,

    key_color: String,
    key_toggle: String,
    key_brightness_down: String,
    key_brightness_up: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rgb_hold_delay_ms: 300,
            rgb_step_delay_ms: 18,
            rgb_hue_step: 5,

            // Defaults tuned for the main keyboard:
            //
            // Ctrl+Alt+/      color / RGB scroll
            // Ctrl+Alt+*      toggle on/off
            // Ctrl+Alt+-      brightness down
            // Ctrl+Alt++      brightness up
            //
            // Important:
            // "*" is usually Shift+8, so we store it as ctrl+alt+8.
            // "+" is usually Shift+=, so we store it as ctrl+alt+equal.
            key_color: "ctrl+alt+slash".to_string(),
            key_toggle: "ctrl+alt+8".to_string(),
            key_brightness_down: "ctrl+alt+minus".to_string(),
            key_brightness_up: "ctrl+alt+equal".to_string(),
        }
    }
}

#[derive(Debug, Default, Clone)]
struct CaptureState {
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
    pressed_non_mods: BTreeSet<u16>,
    last_combo: Option<String>,
    last_event_at: Option<Instant>,
}

fn must_be_root() {
    if unsafe { geteuid() } != 0 {
        eprintln!("Run as root:");
        eprintln!("  sudo machenike-config");
        exit(1);
    }
}

fn read_line(prompt: &str) -> String {
    print!("{prompt}");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input.trim().to_string()
}

fn pause() {
    let _ = read_line("\nPress Enter to continue...");
}

fn clear() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap();
}

fn key_code_to_name(code: u16) -> String {
    match code {
        KEY_1 => "1".to_string(),
        KEY_2 => "2".to_string(),
        KEY_3 => "3".to_string(),
        KEY_4 => "4".to_string(),
        KEY_5 => "5".to_string(),
        KEY_6 => "6".to_string(),
        KEY_7 => "7".to_string(),
        KEY_8 => "8".to_string(),
        KEY_9 => "9".to_string(),
        KEY_0 => "0".to_string(),

        KEY_Q => "q".to_string(),
        KEY_W => "w".to_string(),
        KEY_E => "e".to_string(),
        KEY_R => "r".to_string(),
        KEY_T => "t".to_string(),
        KEY_Y => "y".to_string(),
        KEY_U => "u".to_string(),
        KEY_I => "i".to_string(),
        KEY_O => "o".to_string(),
        KEY_P => "p".to_string(),

        KEY_A => "a".to_string(),
        KEY_S => "s".to_string(),
        KEY_D => "d".to_string(),
        KEY_F => "f".to_string(),
        KEY_G => "g".to_string(),
        KEY_H => "h".to_string(),
        KEY_J => "j".to_string(),
        KEY_K => "k".to_string(),
        KEY_L => "l".to_string(),

        KEY_Z => "z".to_string(),
        KEY_X => "x".to_string(),
        KEY_C => "c".to_string(),
        KEY_V => "v".to_string(),
        KEY_B => "b".to_string(),
        KEY_N => "n".to_string(),
        KEY_M => "m".to_string(),

        KEY_SLASH => "slash".to_string(),
        KEY_MINUS => "minus".to_string(),
        KEY_EQUAL => "equal".to_string(),
        KEY_SPACE => "space".to_string(),

        KEY_KPSLASH => "kpslash".to_string(),
        KEY_KPASTERISK => "kpasterisk".to_string(),
        KEY_KPMINUS => "kpminus".to_string(),
        KEY_KPPLUS => "kpplus".to_string(),

        _ => format!("code:{code}"),
    }
}

fn combo_display(combo: &str) -> String {
    let mut out = combo.to_string();

    out = out.replace("ctrl", "Ctrl");
    out = out.replace("alt", "Alt");
    out = out.replace("shift", "Shift");
    out = out.replace("meta", "Meta");

    out = out.replace("slash", "/");
    out = out.replace("minus", "-");
    out = out.replace("equal", "+");
    out = out.replace("space", "Space");

    out = out.replace("kpasterisk", "KP*");
    out = out.replace("kpslash", "KP/");
    out = out.replace("kpminus", "KP-");
    out = out.replace("kpplus", "KP+");

    // For nicer display:
    // ctrl+alt+8 is our default for Ctrl+Alt+* on the main keyboard.
    out = out.replace("Ctrl+Alt+8", "Ctrl+Alt+*");

    out
}

fn read_config() -> Config {
    let mut config = Config::default();

    let Ok(content) = fs::read_to_string(CONFIG_FILE) else {
        return config;
    };

    for raw_line in content.lines() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let value = value.trim();

        match key {
            "rgb_hold_delay_ms" => {
                if let Ok(v) = value.parse::<u64>() {
                    config.rgb_hold_delay_ms = v.clamp(0, 5000);
                }
            }

            "rgb_step_delay_ms" => {
                if let Ok(v) = value.parse::<u64>() {
                    config.rgb_step_delay_ms = v.clamp(5, 1000);
                }
            }

            "rgb_hue_step" => {
                if let Ok(v) = value.parse::<u16>() {
                    config.rgb_hue_step = v.clamp(1, 120);
                }
            }

            "key_color" => config.key_color = value.to_string(),
            "key_toggle" => config.key_toggle = value.to_string(),
            "key_brightness_down" => config.key_brightness_down = value.to_string(),
            "key_brightness_up" => config.key_brightness_up = value.to_string(),

            _ => {}
        }
    }

    config
}

fn save_config(config: &Config) {
    fs::create_dir_all(CONFIG_DIR).unwrap_or_else(|error| {
        eprintln!("Failed to create {CONFIG_DIR}: {error}");
        exit(1);
    });

    let content = format!(
        r#"# MACHENIKE hotkey daemon config

# RGB scroll behavior.
rgb_hold_delay_ms={}
rgb_step_delay_ms={}
rgb_hue_step={}

# Hotkeys.
# Format examples:
# ctrl+alt+slash
# ctrl+alt+shift+k
# alt+space
# meta+f
# ctrl+alt+code:53
#
# Notes:
# - "*" on the main keyboard is usually Shift+8.
#   Default toggle is stored as ctrl+alt+8 so Ctrl+Alt+* works too.
# - "+" on the main keyboard is usually Shift+=.
#   Default brightness-up is stored as ctrl+alt+equal so Ctrl+Alt++ works too.

key_color={}
key_toggle={}
key_brightness_down={}
key_brightness_up={}
"#,
        config.rgb_hold_delay_ms,
        config.rgb_step_delay_ms,
        config.rgb_hue_step,
        config.key_color,
        config.key_toggle,
        config.key_brightness_down,
        config.key_brightness_up,
    );

    fs::write(CONFIG_FILE, content).unwrap_or_else(|error| {
        eprintln!("Failed to write {CONFIG_FILE}: {error}");
        exit(1);
    });
}

fn run_systemctl(args: &[&str]) {
    let status = Command::new("systemctl").args(args).status();

    match status {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("systemctl {} failed: {status}", args.join(" ")),
        Err(error) => eprintln!("failed to run systemctl: {error}"),
    }
}

fn restart_daemon() {
    println!("Restarting {SERVICE}...");

    run_systemctl(&["daemon-reload"]);
    run_systemctl(&["restart", SERVICE]);
    run_systemctl(&["start", SERVICE]);

    println!("Done.");
}

fn print_config(config: &Config) {
    println!("Current config:");
    println!();
    println!("  RGB hold delay:       {} ms", config.rgb_hold_delay_ms);
    println!("  RGB step delay:       {} ms", config.rgb_step_delay_ms);
    println!("  RGB hue step:         {}", config.rgb_hue_step);
    println!();
    println!("  Color / RGB scroll:   {}", combo_display(&config.key_color));
    println!("  Toggle on/off:        {}", combo_display(&config.key_toggle));
    println!("  Brightness down:      {}", combo_display(&config.key_brightness_down));
    println!("  Brightness up:        {}", combo_display(&config.key_brightness_up));
}

fn choose_speed(config: &mut Config) {
    clear();

    println!("RGB scroll speed");
    println!();
    println!("1) Slow");
    println!("2) Normal");
    println!("3) Fast    current default");
    println!("4) Very fast");
    println!("5) Custom");
    println!("0) Back");
    println!();

    let choice = read_line("Choose: ");

    match choice.as_str() {
        "1" => {
            config.rgb_step_delay_ms = 35;
            config.rgb_hue_step = 3;
        }
        "2" => {
            config.rgb_step_delay_ms = 25;
            config.rgb_hue_step = 4;
        }
        "3" => {
            config.rgb_step_delay_ms = 18;
            config.rgb_hue_step = 5;
        }
        "4" => {
            config.rgb_step_delay_ms = 12;
            config.rgb_hue_step = 7;
        }
        "5" => {
            let delay = read_line("rgb_step_delay_ms, lower = faster, default 18: ");
            let step = read_line("rgb_hue_step, higher = faster, default 5: ");

            if let Ok(v) = delay.parse::<u64>() {
                config.rgb_step_delay_ms = v.clamp(5, 1000);
            }

            if let Ok(v) = step.parse::<u16>() {
                config.rgb_hue_step = v.clamp(1, 120);
            }
        }
        "0" => return,
        _ => {
            println!("Invalid choice.");
            pause();
            return;
        }
    }

    save_config(config);
    restart_daemon();
    pause();
}

fn choose_hold_delay(config: &mut Config) {
    clear();

    println!("Hold delay");
    println!();
    println!("Current: {} ms", config.rgb_hold_delay_ms);
    println!("Default: 300 ms");
    println!();

    let input = read_line("New hold delay in ms: ");

    match input.parse::<u64>() {
        Ok(v) => {
            config.rgb_hold_delay_ms = v.clamp(0, 5000);
            save_config(config);
            restart_daemon();
        }
        Err(_) => println!("Invalid number."),
    }

    pause();
}

fn read_input_event(file: &mut fs::File) -> io::Result<(u16, u16, i32)> {
    let mut buf = [0u8; 24];
    file.read_exact(&mut buf)?;

    Ok((
        u16::from_ne_bytes([buf[16], buf[17]]),
        u16::from_ne_bytes([buf[18], buf[19]]),
        i32::from_ne_bytes([buf[20], buf[21], buf[22], buf[23]]),
    ))
}

fn update_capture_state(state: &mut CaptureState, code: u16, value: i32) {
    let pressed = value != 0;

    match code {
        KEY_LEFTCTRL | KEY_RIGHTCTRL => state.ctrl = pressed,
        KEY_LEFTALT | KEY_RIGHTALT => state.alt = pressed,
        KEY_LEFTSHIFT | KEY_RIGHTSHIFT => state.shift = pressed,
        KEY_LEFTMETA | KEY_RIGHTMETA => state.meta = pressed,

        _ => {
            if value == 1 {
                state.pressed_non_mods.insert(code);
            } else if value == 0 {
                state.pressed_non_mods.remove(&code);
            }
        }
    }
}

fn build_combo_from_state(state: &CaptureState, key_code: u16) -> String {
    let mut parts = Vec::new();

    if state.ctrl {
        parts.push("ctrl".to_string());
    }
    if state.alt {
        parts.push("alt".to_string());
    }
    if state.shift {
        parts.push("shift".to_string());
    }
    if state.meta {
        parts.push("meta".to_string());
    }

    parts.push(key_code_to_name(key_code));

    parts.join("+")
}

fn capture_hotkey_from_keyboard() -> Option<String> {
    println!("Press the desired key combination now.");
    println!();
    println!("Then release it and do not press anything for 2 seconds.");
    println!("The last captured combination will be saved.");
    println!();
    println!("Examples:");
    println!("  Ctrl+Alt+/");
    println!("  Ctrl+Alt+K");
    println!("  Ctrl+Shift+M");
    println!("  Alt+Space");
    println!("  Meta+F");
    println!();
    println!("Waiting...");

    let (tx, rx) = mpsc::channel::<(u16, i32)>();

    let entries = match fs::read_dir("/dev/input") {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("Failed to read /dev/input: {error}");
            return None;
        }
    };

    let mut device_count = 0;

    for entry in entries.flatten() {
        let path = entry.path();

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !name.starts_with("event") {
            continue;
        }

        device_count += 1;
        let tx = tx.clone();

        thread::spawn(move || {
            let Ok(mut file) = fs::File::open(&path) else {
                return;
            };

            loop {
                match read_input_event(&mut file) {
                    Ok((event_type, code, value)) => {
                        if event_type == EV_KEY {
                            let _ = tx.send((code, value));
                        }
                    }
                    Err(_) => return,
                }
            }
        });
    }

    drop(tx);

    if device_count == 0 {
        println!("No /dev/input/event* devices found.");
        return None;
    }

    let mut state = CaptureState::default();
    let global_start = Instant::now();

    loop {
        if global_start.elapsed() > Duration::from_secs(30) {
            println!("Timeout. No combo saved.");
            return None;
        }

        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok((code, value)) => {
                update_capture_state(&mut state, code, value);
                state.last_event_at = Some(Instant::now());

                if value == 1 {
                    match code {
                        KEY_LEFTCTRL
                        | KEY_RIGHTCTRL
                        | KEY_LEFTALT
                        | KEY_RIGHTALT
                        | KEY_LEFTSHIFT
                        | KEY_RIGHTSHIFT
                        | KEY_LEFTMETA
                        | KEY_RIGHTMETA => {}

                        _ => {
                            let combo = build_combo_from_state(&state, code);
                            println!("Captured candidate: {}", combo_display(&combo));
                            state.last_combo = Some(combo);
                        }
                    }
                }
            }

            Err(mpsc::RecvTimeoutError::Timeout) => {
                let Some(last_event_at) = state.last_event_at else {
                    continue;
                };

                if last_event_at.elapsed() >= Duration::from_secs(2) {
                    if let Some(combo) = state.last_combo.clone() {
                        println!();
                        println!("Saved combo: {}", combo_display(&combo));
                        return Some(combo);
                    }

                    println!("No non-modifier key was captured.");
                    return None;
                }
            }

            Err(mpsc::RecvTimeoutError::Disconnected) => {
                println!("Input capture disconnected.");
                return None;
            }
        }
    }
}

fn choose_one_hotkey(title: &str, current: &str) -> Option<String> {
    clear();

    println!("{title}");
    println!();
    println!("Current: {}", combo_display(current));
    println!();
    println!("1) Record hotkey by pressing it");
    println!("2) Type combo manually");
    println!("0) Back");
    println!();

    let choice = read_line("Choose: ");

    match choice.as_str() {
        "1" => {
            println!();
            capture_hotkey_from_keyboard()
        }

        "2" => {
            println!();
            println!("Examples:");
            println!("  ctrl+alt+slash");
            println!("  ctrl+alt+k");
            println!("  ctrl+shift+m");
            println!("  alt+space");
            println!("  meta+f");
            println!("  ctrl+alt+code:53");
            println!();

            let value = read_line("Combo: ");

            if value.trim().is_empty() {
                None
            } else {
                Some(value.trim().to_lowercase())
            }
        }

        "0" => None,

        _ => {
            println!("Invalid choice.");
            pause();
            None
        }
    }
}

fn configure_hotkeys(config: &mut Config) {
    loop {
        clear();

        println!("Hotkeys");
        println!();
        println!("1) Color / RGB scroll   {}", combo_display(&config.key_color));
        println!("2) Toggle on/off        {}", combo_display(&config.key_toggle));
        println!("3) Brightness down      {}", combo_display(&config.key_brightness_down));
        println!("4) Brightness up        {}", combo_display(&config.key_brightness_up));
        println!("0) Back");
        println!();

        let choice = read_line("Choose: ");

        match choice.as_str() {
            "1" => {
                if let Some(v) = choose_one_hotkey("Color / RGB scroll hotkey", &config.key_color) {
                    config.key_color = v;
                    save_config(config);
                    restart_daemon();
                    pause();
                }
            }

            "2" => {
                if let Some(v) = choose_one_hotkey("Toggle on/off hotkey", &config.key_toggle) {
                    config.key_toggle = v;
                    save_config(config);
                    restart_daemon();
                    pause();
                }
            }

            "3" => {
                if let Some(v) =
                    choose_one_hotkey("Brightness down hotkey", &config.key_brightness_down)
                {
                    config.key_brightness_down = v;
                    save_config(config);
                    restart_daemon();
                    pause();
                }
            }

            "4" => {
                if let Some(v) =
                    choose_one_hotkey("Brightness up hotkey", &config.key_brightness_up)
                {
                    config.key_brightness_up = v;
                    save_config(config);
                    restart_daemon();
                    pause();
                }
            }

            "0" => break,

            _ => {
                println!("Invalid choice.");
                pause();
            }
        }
    }
}

fn restore_defaults(config: &mut Config) {
    clear();

    println!("Restore default settings");
    println!("========================");
    println!();
    println!("This will reset hotkeys and RGB scroll settings to defaults.");
    println!();
    println!("Default hotkeys:");
    println!("  Color / RGB scroll   Ctrl+Alt+/");
    println!("  Toggle on/off        Ctrl+Alt+*");
    println!("  Brightness down      Ctrl+Alt+-");
    println!("  Brightness up        Ctrl+Alt++");
    println!();
    println!("Default RGB scroll:");
    println!("  Hold delay           300 ms");
    println!("  Step delay           18 ms");
    println!("  Hue step             5");
    println!();
    println!("Press Enter to restore defaults.");
    println!("Press Ctrl+C to cancel.");
    println!();

    let _ = read_line("Restore defaults now? ");

    *config = Config::default();
    save_config(config);
    restart_daemon();

    println!();
    println!("Default settings restored.");
    println!();
    println!("Current defaults:");
    println!("  Color / RGB scroll   Ctrl+Alt+/");
    println!("  Toggle on/off        Ctrl+Alt+*");
    println!("  Brightness down      Ctrl+Alt+-");
    println!("  Brightness up        Ctrl+Alt++");

    pause();
}

fn show_status(config: &Config) {
    clear();

    print_config(config);

    println!();
    println!("Service status:");
    println!();

    let _ = Command::new("systemctl")
        .args(["status", SERVICE, "--no-pager"])
        .status();

    pause();
}

fn main_menu() {
    let mut config = read_config();

    loop {
        clear();

        println!("MACHENIKE Linux Control");
        println!("=======================");
        println!();

        print_config(&config);

        println!();
        println!("1) RGB scroll speed");
        println!("2) Hold delay");
        println!("3) Hotkeys");
        println!("4) Restore defaults");
        println!("5) Restart daemon");
        println!("6) Status");
        println!("0) Exit");
        println!();

        let choice = read_line("Choose: ");

        match choice.as_str() {
            "1" => choose_speed(&mut config),
            "2" => choose_hold_delay(&mut config),
            "3" => configure_hotkeys(&mut config),
            "4" => restore_defaults(&mut config),
            "5" => {
                restart_daemon();
                pause();
            }
            "6" => show_status(&config),
            "0" => break,
            _ => {
                println!("Invalid choice.");
                pause();
            }
        }

        config = read_config();
    }
}

fn main() {
    must_be_root();

    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("defaults") => {
            let config = Config::default();
            save_config(&config);
            restart_daemon();
            println!("Defaults restored.");
            println!();
            println!("CLI menu:");
            println!("  sudo machenike-config");
        }

        Some("status") => {
            let config = read_config();
            print_config(&config);
        }

        Some("restart") => {
            restart_daemon();
        }

        Some(_) => {
            eprintln!("Usage:");
            eprintln!("  sudo machenike-config");
            eprintln!("  sudo machenike-config defaults");
            eprintln!("  sudo machenike-config status");
            eprintln!("  sudo machenike-config restart");
            exit(1);
        }

        None => main_menu(),
    }
}

#[cfg(target_os = "linux")]
unsafe fn geteuid() -> u32 {
    unsafe extern "C" {
        fn geteuid() -> u32;
    }

    geteuid()
}
