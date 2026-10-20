fn main() -> paperhands::Result<()> {
    let index = clap_how_to::Index::new(paperhands::help_documents())?;
    std::fs::write("assets/how-to.json", serde_json::to_vec(index.prepared())?)?;
    let routes = clap_how_to::Index::new(paperhands::companion::route_documents())?;
    std::fs::write(
        "assets/companion.json",
        serde_json::to_vec(routes.prepared())?,
    )?;
    println!(
        "Prepared {} help sources and {} companion routes",
        index.prepared().documents.len(),
        routes.prepared().documents.len()
    );
    Ok(())
}
