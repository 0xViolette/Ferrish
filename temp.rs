fn main() {
    let mut parts = "1 2 3".split(" ");
    println!("{:?}", parts.next());
    println!("{:#?}", parts.to_vec());
}
