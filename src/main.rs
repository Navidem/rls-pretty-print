extern crate rls_analysis;
extern crate rls_data; 
extern crate serde_json;

use std::{path::Path, env};
use std::process::{Command, Stdio};
use std::io::{Error, ErrorKind};

use rls_analysis::{AnalysisHost, DefKind};
use rls_data::config::Config as AnalysisConfig;

pub fn main() -> Result< (), Box<std::error::Error> >{
    let args: Vec<String> = env::args().collect();
    let analysis = rls_analysis::AnalysisHost::new(rls_analysis::Target::Debug);
    let mut path = Path::new(".");
    if args.len() > 1 {
        path = Path::new(&args[1]);
    } 
    println!("Wroking path: {:?}", path);

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

pub fn traverse(id: rls_analysis::Id, defin: rls_analysis::Def , analysis: &AnalysisHost, mut indent: u32) 
    -> Result < (), Box<std::error::Error>> {
    println!("{}{:?} {:?} {:?}", " ".repeat(indent as usize), id, defin.kind, defin.name);
    match defin.kind {
        DefKind::Function 
        | DefKind::Method => emit_sig(&analysis, &defin, &indent)?,
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

pub fn emit_sig (analysis: &AnalysisHost, defin: &rls_analysis::Def, indent: &u32) -> Result < (), Box<std::error::Error>>{
    let def = defin.clone();
    println!("{}Qualname: {} ", " ".repeat(*indent as usize +2), def.qualname );
    match def.sig {
        Some(x) => {
            println!("{}Signature: {}", " ".repeat(*indent as usize+2), x.text); //defin.value has the text as well
            for sig_el in x.defs{
                let qname = analysis.get_def(sig_el.id)?.qualname;
                println!("{}defs: id: {}, qualname: {}", " ".repeat(*indent as usize+4), sig_el.id, qname);
            }
            for sig_el in x.refs{
                let qname = analysis.get_def(sig_el.id)?.qualname;
                println!("{}refs: id: {}, qualname: {}", " ".repeat(*indent as usize+4), sig_el.id, qname);
            }
               // println!("{:?}", defin.sig);
        }
        None => println!("{}Signature (value): {}", " ".repeat(*indent as usize+2), def.value),
    }
    Ok(())
}
fn generate_analysis_files(dir : &Path) -> Result <(), Box<std::error::Error> >{
    let mut command = Command::new("cargo");

    let target_dir = dir.join("target").join("rls");
    let manifest_path = dir.join("Cargo.toml");

    let metadata = retrieve_metadata(&manifest_path)?;
    let target = target_from_metadata(&metadata)?;
    let analysis_config = AnalysisConfig {
        //full_docs: true,
       // pub_only: true,
        signatures: true,
        ..Default::default()
    };

    command
        .arg("check")
        .arg("--manifest-path")
        .arg(manifest_path)
        .env("RUSTFLAGS", "-Z save-analysis")
        .env("CARGO_TARGET_DIR", target_dir)
        //RUST_SAVE_ANALYSIS_CONFIG=' "reachable_only": true, "full_docs": true, "pub_only": false, 
            //"distro_crate": false, "signatures": false, "borrow_data": false'
        .env("RUST_SAVE_ANALYSIS_CONFIG", serde_json::to_string(&analysis_config)?,)
        .stderr(Stdio::piped())
        .stdout(Stdio::null());
    
    //command.current_dir(dir);
    match target.kind {
        TargetKind::Library => {
            command.arg("--lib");
        }
        TargetKind::Binary => {
            command.args(&["--bin", &target.name]);
        }
    } 
    //command.args(&["rustc", "--lib", "--", "-Z", "save-analysis"]);
    //command.arg("--lib");
    println!("Generating rls analysis data ...");
    println!("{:?}", command );
    let mut child = command.spawn()?;

    let status = child.wait()?;

    if !status.success() {
        println!("ERROR!" );
        println!("{:?}", command );        
        println!("child process spawned: {:?}", status);
        return Err(Box::new(Error::new(ErrorKind::Other, "Child Process error")))
    }
    Ok(())

}
 
///codes from doxidize
pub fn retrieve_metadata(manifest_path: &Path) -> Result<serde_json::Value,  Box<std::error::Error>> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--no-deps")
        .arg("--format-version")
        .arg("1")
        .output()?;

    if !output.status.success() {
        println!("ERROR!" );
        println!("{:?}", output );  
        //return Err();
    }

    Ok(serde_json::from_slice(&output.stdout)?)
}


/// Parse the library target from the crate metadata.
pub fn target_from_metadata(metadata: &serde_json::Value) -> Result<Target, Box<std::error::Error>> {
    // We can expect at least one package and target, otherwise the metadata generation would have
    // failed.
    let targets = metadata["packages"][0]["targets"]
        .as_array()
        .expect("`targets` is not an array");

    let mut targets = targets
        .into_iter()
        .flat_map(|target| {
            let name = target["name"].as_str().expect("`name` is not a string");
            let kinds = target["kind"].as_array().expect("`kind` is not an array");

            if kinds.len() != 1 {
                println!("expected one kind for target '{}'", name);
                return Some(Err(Error::new(ErrorKind::Other, "OOps!")));
            }

            let kind = match kinds[0].as_str().unwrap() {
                "lib" => TargetKind::Library,
                "bin" => TargetKind::Binary,
                _ => return None,
            };

            let target = Target {
                name: name.to_owned(),
                kind,
            };

            Some(Ok(target))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if targets.is_empty() {
        println!("no targets with supported kinds (`bin`, `lib`) found" );
        Err(Box::new(Error::new(ErrorKind::Other, "OOps!")))
    } else if targets.len() == 1 {
        Ok(targets.remove(0))
    } else {
        // FIXME(#105): Handle more than one target.
        let (mut libs, mut bins): (Vec<_>, Vec<_>) =
            targets.into_iter().partition(|target| match target.kind {
                TargetKind::Library => true,
                TargetKind::Binary => false,
            });

        // Default to documenting the library if it exists.
        let target = if !libs.is_empty() {
            libs.remove(0)
        } else {
            bins.remove(0)
        };

        let kind = match target.kind {
            TargetKind::Library => "library",
            TargetKind::Binary => "first binary",
        };

        println!(
            "Found more than one target to document. Documenting the {}: {}", kind, target.name
        );

        Ok(target)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TargetKind {
    /// A `bin` target.
    Binary,

    /// A `lib` target.
    Library,
}


/// A target of documentation.
#[derive(Debug, PartialEq, Eq)]
pub struct Target {
    /// The kind of the target.
    pub kind: TargetKind,

    /// The name of the target.
    ///
    /// This is *not* the name of the target's crate, which is used to retrieve the analysis data.
    /// Use the [`crate_name`] method instead.
    ///
    /// [`crate_name`]: ./struct.Target.html#method.crate_name
    pub name: String,
}

impl Target {
    /// Returns the name of the target's crate.
    ///
    /// This name is equivalent to the target's name, with dashes replaced by underscores.
    pub fn crate_name(&self) -> String {
        self.name.replace('-', "_")
    }
}