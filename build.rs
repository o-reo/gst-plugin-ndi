use std::env;

fn main()
{
    let ndi_path = env::var("NDI_PATH");
    if ndi_path.is_ok() {
        let ndi_path = ndi_path.unwrap();
        println!("cargo:rustc-link-search={}", ndi_path);
        println!("cargo:rustc-link-args=-W1,-rpath,{}", ndi_path);
        println!("cargo:rustc-link-lib=ndi");
    }
}
