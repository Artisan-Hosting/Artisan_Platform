use ais_common::{system::{get_machine_id, get_system_stats}, version::Version};
use lsb_release::LsbRelease;
use simple_pretty::output;

fn main() {
    let system = get_system_stats();
    let lsb_failsafe: LsbRelease = LsbRelease {
        id: String::from("failsafe"),
        desc: String::from("System in a damaged state"),
        version: Version::get(),
        code_name: String::from("Wacky Whitfield"),
    };

    let ais_version: Version = Version::get_raw();
    let ais_identyfi: String = get_machine_id();
    let system_version: LsbRelease = lsb_release::info().unwrap_or(lsb_failsafe);
    let system_hostname = gethostname::gethostname();
    // let (system_load_1, system_load_5, system_load_15) = match sys.load_average() {
    //     Ok(l) => (l.one, l.five, l.fifteen),

    //     Err(_) => {
    //         let val: f32 = 0.0;
    //         (val, val, val)
    //     }
    // };

    let welcome_text = format!(
        r#"
                  _    _                         _    _                   _
     /\          | |  (_)                       | |  | |                 (_) 
    /  \    _ __ | |_  _  ___   __ _  _ __      | |__| |  ___   ___ | |_     _ __    __ _
   / /\ \  | '__|| __|| |/ __| / _` || '_ \     | '__' | / _ \ /`__|| __|| || '_ \  / _` |
  / ____ \ | |   | |_ | |\__ \| (_| || | | |    | |  | || (_) |\__ \| |_ | || | | || (_| |
 /_/    \_\|_|    \__||_||___/ \__,_||_| |_|    |_|  |_| \___/ |___/ \__||_||_| |_| \__, |
                                                                                     __/ |
                                                                                    |___/   
 
Your machine at a glance:

Os Version   : {}
AIS Version  : {}
AIS id       : {}
Hostname     : {:?}
Mem Usage    : {:.4}M

Welcome!

This server is hosted by Artisan Hosting. If you're reading this now would probably be a good time 
to contact me at dwhitfield@artisanhosting.net or shoot me a text at 414-578-0988. Thank you for
supporting me and Artisan Hosting.

"#,
        format!("{} - {}", system_version.version, system_version.code_name),
        format!("{}_{}", ais_version.number, ais_version.code),
        ais_identyfi.trim_end(),
        system_hostname,
        // system_load_1,
        // system_load_5,
        // system_load_15,
        system.get("Used RAM").unwrap_or(&"X.xx".to_owned())
    );

    output("BLUE", &format!("{}", welcome_text));
}

// System Load  : {:.2}, {:.2}, {:.2}