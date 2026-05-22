// build.rs
fn main() {
  // Tell Cargo that if the given file or directory changes,
  // it should rerun this build script and recompile the project.
  println!("cargo:rerun-if-changed=config/databases");
}
