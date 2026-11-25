use lsp_server::Connection;

fn main() -> anyhow::Result<()> {
    let (connection, io_threads) = Connection::stdio();
    jes_reader_lsp::run(&connection)?;
    drop(connection);
    io_threads.join()?;
    Ok(())
}
