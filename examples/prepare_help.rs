fn main() -> paperhands::Result<()> {
    let index = clap_how_to::Index::new(paperhands::help_documents())?;
    std::fs::write("assets/how-to.json", serde_json::to_vec(index.prepared())?)?;
    println!("Prepared {} help sources", index.prepared().documents.len());
    Ok(())
}
