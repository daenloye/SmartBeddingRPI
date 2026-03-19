use chrono::Local;

// Al poner 'pub', permitimos que otros módulos la usen
pub fn logger(module: &str, msg: &str) {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("[{}] [{}] {}", now, module, msg);
}