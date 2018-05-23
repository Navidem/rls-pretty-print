extern crate rls_analysis;
use std::path;
use rls_analysis::AnalysisLoader;
fn main() -> Result< (), Box<std::error::Error> >{

    test();

   // let mut ld = rls_analysis::CargoAnalysisLoader::new(rls_analysis::Target::Debug);
   // ld.set_path_prefix(path::Path::new("."));
    //let host = ld.fresh_host();

    let analysis = rls_analysis::AnalysisHost::new(rls_analysis::Target::Debug);
    let _roots = analysis.def_roots()?;
    Ok(())

}

fn test () -> i32{
    let i = 2;
    return i
}