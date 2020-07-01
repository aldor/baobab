use argparse;

#[derive(Debug)]
pub struct Args {
    pub url: String,
}

pub fn parse_args() -> Args {
    let mut args = Args {
        url: String::default(),
    };
    {
        let mut parser = argparse::ArgumentParser::new();
        parser.refer(&mut args.url).required().add_argument(
            "url",
            argparse::Store,
            "build url to watch",
        );
        parser.parse_args_or_exit();
    }
    args
}
