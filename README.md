# Algol
Algol is a Discord bot designed to facilitate a game of "Assassin" where players are assigned targets and must "eliminate" them by following specific instructions. The game continues until only one player remains.

This bot is designed to select a target for each player so that all the players for a unique and closed loop, and so that a player doesn't have or can't be killed by their own instructions. This means that there is a minimum of 4 players required.

## Features
- **Sign Up**: Players can sign up for the game with a username and a kill instruction.
- **Admin Commands**: The first player to sign up becomes the admin and can manage the game.
- **Game Management**: Admin can start the game, set the announcement channel, and reveal game details.
- **Player Commands**: Players can check their targets, change their information, and report kills.
- **Private Messaging**: Players receive their targets and instructions via private messages.

## Commands
### Admin Commands
- `/add_player`: Add a player to the game.
  - _id_: User ID of the player.
  - _name_: Player's name.
  - _instruction_: Kill instruction for the player.
- `/set_channel`: Set the channel where game announcements will be sent.
  - _channel_: Channel ID (optional).
- `/get_channel`: Get the current announcement channel.
- `/start_game`: Start the game if enough players have signed up.
- `/inform`: Inform all players of their targets and kill instructions privately.
- `/reveal`: Reveal all players, their targets, and kill instructions.

### Player Commands
- `/sign_up`: Sign up for the game.
  - _name_: Player's name (optional).
  - _instruction_: Kill instruction (optional).
- `/change_name`: Change your player name.
  - _name_: New player name (optional).
- `/add_instruction`: Add more instructions.
  - _instruction_: Kill instruction (optional).
- `/get_target`: Get your current target and kill instruction.
- `/kill`: Report a successful kill and get your next target.

## Installation
1. **Clone the repository**:
    ```sh
    git clone https://github.com/arnau.delrio/algol.git
    cd algol
    ```

2. **Create a `Secrets.toml` file** in the root directory with your Discord bot token:
    ```toml
    discord_token = "YOUR_DISCORD_BOT_TOKEN"
    ```

3. **Build and run the bot**:
    ```sh
    cargo build --release
    cargo run --release
    ```

If you wish to run the terminal version, without the discord bot, use the following command:
```sh
cargo run term
```

## Usage
1. **Invite the bot to your Discord server** using the OAuth2 URL with the necessary permissions.
2. **Use the commands** listed above to manage and play the game.

## Development
### Prerequisites
- Rust and Cargo installed on your machine.
- A Discord bot token.

## Contributing
Contributions are welcome! Please open an issue or submit a pull request for any changes or improvements.

## License
This project is licensed under the GPLv3 License. See the [LICENSE.md](LICENSE.md) file for details.

## Acknowledgements
- [Serenity](https://github.com/serenity-rs/serenity) - A Rust library for the Discord API.
- [Poise](https://github.com/kangalioo/poise) - A framework for building Discord bots in Rust.
- [Tokio](https://tokio.rs/) - An asynchronous runtime for Rust.
