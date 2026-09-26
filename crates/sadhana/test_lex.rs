use sadhana::lex::{lex, Kind};
fn main() {
    let code = "वृत्तिः क (खॱअ३२) आदि ।";
    println!("{:?}", lex(code));
}
