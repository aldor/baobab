use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "baobab", about = "baobab cli")]
pub struct Args {
    #[structopt(name = "URL")]
    pub url: String,
}

pub fn parse_args() -> Args {
    Args::from_args()
}
