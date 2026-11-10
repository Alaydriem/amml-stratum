use stratum_tools::PreviewPng;

fn main() -> std::io::Result<()> {
    PreviewPng::run(std::env::args().skip(1))
}
