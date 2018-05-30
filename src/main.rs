extern crate rls_analysis;
extern crate rls_data; 
extern crate rls_pretty_print;
extern crate serde_json;

use std::{path, env};
use std::process::{Command, Stdio};

use rls_analysis::{AnalysisHost, DefKind};

pub fn main() -> Result< (), Box<std::error::Error> >{
    let args: Vec<String> = env::args().collect();
    //println!("args len: {}",args.len() );
   // let mut ld = rls_analysis::CargoAnalysisLoader::new(rls_analysis::Target::Debug);
   // ld.set_path_prefix(path::Path::new("."));
    //let host = ld.fresh_host();
    //type Blacklist<'a> = &'a [&'static str];
    let analysis = rls_analysis::AnalysisHost::new(rls_analysis::Target::Debug);
    let mut path = path::Path::new(".");
    if args.len() > 1 {
        path = path::Path::new(&args[1]);
    } 
    println!("Wroking path: {:?}", path);
    //let blacklist : Blacklist;
    //analysis.reload_with_blacklist(path, path, &bckList);
    //analysis.reload_from_analysis(std::vec::Vec<rls_data::Analysis>, path, path, blacklist);

    //path_prefix: Cargo's working directory and will contain the target directory
    //base_dir: is the root of the whole workspace

    generate_analysis_files(path)?;  // necessary to create the save-analysis dir
    analysis.reload(path, path)?;
    let mut roots = analysis.def_roots()?;
    roots.sort_unstable_by(|(_, name1), (_, name2)| name1.cmp(name2));

    for (id, membr_name) in roots {
        let def = analysis.get_def(id)?;
        println!("Root: {:?} {:?} {:?} {}", id, def.kind, def.name, membr_name );
        traverse(id, def , &analysis, 0)?;
    }
    Ok(())



}

fn traverse(id: rls_analysis::Id, defin: rls_analysis::Def ,analysis: &AnalysisHost, mut indent: u32) 
    -> Result < (), Box<std::error::Error>> {
    println!("{}{:?} {:?} {:?}", " ".repeat(indent as usize), id, defin.kind, defin.name);
    match defin.kind {
        DefKind::Function 
        | DefKind::Method => { println!("{}Qualname: {} ", " ".repeat(indent as usize +2), defin.qualname );
            println!("{}Signature: {}", " ".repeat(indent as usize+2), defin.value); },
        _ => (),
    }
    indent += 2;
    let mut children = analysis.for_each_child_def(id, |id, def| (id, def.clone()) )?;
    children.sort_unstable_by(|(_, def1), (_, def2)| def1.name.cmp(&def2.name));
    for (child, def) in children {
        traverse(child, def,  analysis, indent)?;
    }
    Ok(())
}

fn generate_analysis_files(dir : &path::Path) -> Result <(), Box<std::error::Error> >{
    let mut command = Command::new("cargo");

    let target_dir = dir.join("target").join("rls");



    command
        .env("RUSTFLAGS", "-Z save-analysis")
        .env("CARGO_TARGET_DIR", target_dir)
        .stderr(Stdio::piped())
        .stdout(Stdio::null());
    
    command.current_dir(dir);
   /*  match target.kind {
        TargetKind::Library => {
            command.arg("--lib");
        }
        TargetKind::Binary => {
            command.args(&["--bin", &target.name]);
        }
    } */
    command.args(&["rustc", "--lib", "--", "-Z", "save-analysis"]);
    println!("Generating rls analysis data ...");
    let mut child = command.spawn()?;

    let status = child.wait()?;

    if !status.success() {
        println!("ERROR!" );
        println!("{:?}", command );        
        println!("rustc process spawned: {:?}", status);
    }
    Ok(())

}