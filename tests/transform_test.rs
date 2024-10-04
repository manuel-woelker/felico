use libtest_mimic::{Arguments, Trial};
use pretty_assertions::assert_eq;
use yaml_rust::{Yaml, YamlLoader};

pub fn run_transform_test(directory: &str, transform_fn: impl Fn(&str) -> String + Send + 'static + Copy) {
    // Parse command line arguments
    let args = Arguments::from_args();

    let yaml_string = std::fs::read_to_string(directory).expect("read file");
    let mut yaml = YamlLoader::load_from_str(&yaml_string).expect("read yaml");
    let doc = yaml.pop().expect("document");
    let mut hash = doc.into_hash().expect("hash");
    let testcases = hash.remove(&Yaml::String("testcases".to_string())).expect("testcases").into_vec().expect("list");
    let tests: Vec<Trial> = testcases.into_iter().map(move |testcase| {
        let mut testcase = testcase.into_hash().expect("hash");
        let name = testcase.remove(&Yaml::String("name".to_string())).expect("name").into_string().expect("string");
        let input = testcase.remove(&Yaml::String("input".to_string())).expect("input").into_string().expect("string");
        let expected_output = testcase.remove(&Yaml::String("output".to_string())).expect("output").into_string().expect("string");
        Trial::test(name, move || {
            let actual_output = transform_fn(&input);
            assert_eq!(actual_output, expected_output);
            Ok(())
        })
    }).collect();


    // Run all tests and exit the application appropriately.
    libtest_mimic::run(&args, tests).exit();
}