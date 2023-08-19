use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["TournamentAssistantProtos/discord.proto", "TournamentAssistantProtos/models.proto", "TournamentAssistantProtos/packets.proto"], &["TournamentAssistantProtos/"])?;

    Ok(())
}