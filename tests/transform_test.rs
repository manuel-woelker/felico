use libtest_mimic::{Arguments, Trial};
use pretty_assertions::assert_eq;
use felico::infra::result::FelicoResult;
use located_yaml::{Marker, Yaml, YamlElt, YamlLoader};
pub fn run_transform_test(directory: &str, transform_fn: impl Fn(&str, &str) -> FelicoResult<String> + Send + 'static + Copy) {
    // Parse command line arguments
    let args = Arguments::from_args();


    macro_rules! cast {
        ($target: expr, $pat: path) => {
            {
                if let $pat(a) = $target.yaml { // #1
                    a
                } else {
                    panic!(
                        "mismatch variant when cast to {}",
                        stringify!($pat)); // #2
                }
            }
        };
    }
    let paths = std::fs::read_dir(directory).unwrap();
    let mut tests: Vec<Trial> = vec![];
    for file in paths {
        let file = file.unwrap();
        let yaml_string = std::fs::read_to_string(file.path()).expect("read file");


        let mut yaml = YamlLoader::load_from_str(&yaml_string).expect("read yaml");
        let doc = yaml.docs.pop().expect("document");
        let mut hash = cast!(doc, YamlElt::Hash);

        let make_key = |key: &str| -> Yaml {
            Yaml {
                yaml: YamlElt::String(key.to_string()),
                marker: Marker {
                    index: 0,
                    line: 0,
                    col: 0,
                },
            }
        };
        let testcases = cast!(hash.remove(&make_key("testcases")).expect("testcases"), YamlElt::Array);
        let name_key = &make_key("name");
        let input_key = &make_key("input");
        let output_key = &make_key("output");

        let filename = file.path().into_os_string().into_string().unwrap();
        let prefix = file.path().file_stem().unwrap().to_str().unwrap().to_string();
        for testcase in testcases {
            let filename = filename.clone();
            let marker = testcase.marker;
            let mut testcase = cast!(testcase, YamlElt::Hash);
            let name = cast!(testcase.remove(name_key).expect("name"), YamlElt::String).replace(" ", "_");
            let input = cast!(testcase.remove(input_key).expect("input"), YamlElt::String);
            let expected_output = cast!(testcase.remove(output_key).expect("output"), YamlElt::String);
            tests.push(Trial::test(format!("{}::{}",prefix, name), move || {
                let actual_output = transform_fn(&name, &input)?;
                assert_eq!(actual_output, expected_output, "Transformation test failed for '{}' \n\t at {}:{}", name, &filename, marker.line);
                Ok(())
            }));
        }
    }

    // Run all tests and exit the application appropriately.
    libtest_mimic::run(&args, tests).exit();
}