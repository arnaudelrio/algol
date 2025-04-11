use std::env;
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;

use ::serenity::all::{ChannelId, CreateMessage, Message, UserId};
use poise::{serenity_prelude as serenity, Context, FrameworkError};
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;
use std::io::{self, Read, Write};

type Error = Box<dyn std::error::Error + Send + Sync>;
type BotContext<'a> = Context<'a, GameData, Error>;

#[derive(Debug, PartialEq, Eq)]
enum States {
    Waiting,
    Playing(i32),
    Finished,
}

impl States {
    fn is_playing(game_state: &mut States) -> bool {
        match game_state {
            States::Playing(_) => true,
            _ => false,
        }
    }
}

// User data struct for Poise
#[derive(Debug)]
struct GameData {
    admin_id: Arc<Mutex<Option<UserId>>>,
    game_state: Arc<Mutex<Option<States>>>,
    channel_id: Arc<Mutex<Option<ChannelId>>>,
    signups: Arc<Mutex<Option<Vec<Player>>>>,
    instructions: Arc<Mutex<Option<Vec<Instructions>>>>,
    objectives: Arc<Mutex<Option<Vec<Objective>>>>,
}

#[derive(Clone, PartialEq, Eq)]
struct Player {
    id: u64,
    name: String,
}

impl Player {
    fn new(id: Option<u64>, name: String) -> Player {
        Player {
            id: match id {
                Some(n) => n,
                None => 0,
            },
            name,
        }
    }

    async fn is_admin(ctx: BotContext<'_>) -> Result<bool, Error> {
        let admin_id = ctx.data().admin_id.lock().await;
        match *admin_id {
            Some(id) => Ok(id == ctx.author().id),
            None => {
                ctx.say("You are not authorized to use this command.")
                    .await?;
                return Ok(false);
            }
        }
    }
}

impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player").field("name", &self.name).finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Objective {
    id: u64,
    player: Player,
    target: Player,
    instructions: Option<Instructions>,
    completed: bool,
}

impl Objective {
    fn new(
        id: Option<u64>,
        player: Player,
        target: Player,
        instructions: Option<Instructions>,
    ) -> Objective {
        Objective {
            id: match id {
                Some(n) => n,
                None => 0,
            },
            player,
            target,
            instructions,
            completed: false,
        }
    }

    async fn complete(id: UserId, ctx: BotContext<'_>) -> Objective {
        let mut objectives = ctx.data().objectives.lock().await;

        let mut current_player_objective: Objective = objectives
            .as_mut()
            .unwrap()
            .iter()
            .find(|p| p.id == id.get())
            .unwrap()
            .clone();

        current_player_objective.completed = true;
        Objective::get_objective(id, ctx).await
    }

    async fn get_objective(id: UserId, ctx: BotContext<'_>) -> Objective {
        let mut objectives = ctx.data().objectives.lock().await;
        let mut target_objective: Objective = objectives
            .as_mut()
            .unwrap()
            .iter()
            .find(|p| p.id == id.get())
            .unwrap()
            .clone();
        while target_objective.completed {
            target_objective = objectives
                .as_mut()
                .unwrap()
                .iter()
                .find(|p| p.player == target_objective.target)
                .unwrap()
                .clone();
        }
        return target_objective;
    }
}

impl std::fmt::Debug for Objective {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Objective")
            .field("player", &self.player)
            .field("target", &self.target)
            .field("instructions", &self.instructions)
            .field("completed", &self.completed)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Instructions {
    id: u64,
    instructions: String,
}

impl Instructions {
    fn new(id: Option<u64>, instructions: String) -> Instructions {
        Instructions {
            id: match id {
                Some(n) => n,
                None => 0,
            },
            instructions,
        }
    }
}

impl std::fmt::Debug for Instructions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Instructions")
            .field(
                &format!("({:#?}) instructions: {:#?}", &self.id, &self.instructions),
                &self.instructions,
            )
            .finish()
    }
}

#[derive(Deserialize)]
struct Secrets {
    discord_token: String,
}

async fn on_error(error: FrameworkError<'_, GameData, Error>) {
    match error {
        FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command {}: {:?}", ctx.command().name, error);
        }
        FrameworkError::EventHandler { error, event, .. } => {
            println!(
                "Error in event handler {}: {:?}",
                event.snake_case_name(),
                error
            );
        }
        other => {
            println!("Other error: {:?}", other);
        }
    }
}

async fn not_started_yet(ctx: BotContext<'_>) -> Result<(), Error> {
    ctx.say("Game has not started yet! Ask the admin to start the game when ready.")
        .await?;
    Ok(())
}

async fn signup(
    ctx: BotContext<'_>,
    id: Option<u64>,
    name: Option<String>,
    instruction: Option<String>,
) -> Result<(), Error> {
    let mut signups = ctx.data().signups.lock().await;
    let mut instructions = ctx.data().instructions.lock().await;
    let mut admin = ctx.data().admin_id.lock().await;
    let mut game_state = ctx.data().game_state.lock().await;

    if States::is_playing(game_state.as_mut().unwrap()) {
        ctx.say("Game already started. Cannot sign up.").await?;
        return Ok(());
    } else if *game_state == Some(States::Finished) {
        ctx.say("Game already finished. Cannot sign up. Ask the admin to start a new game.")
            .await?;
        return Ok(());
    }

    if signups.as_mut().unwrap().len() == 0 {
        *admin = Some(UserId::new(id.unwrap()));
        ctx.say("You are now the admin player!").await?;
    } else if signups
        .as_mut()
        .unwrap()
        .iter()
        .any(|c| c.id == id.unwrap())
    {
        ctx.say(format!(
            "You are already signed up with username {}.",
            signups
                .as_mut()
                .unwrap()
                .iter()
                .find(|c| c.id == id.unwrap())
                .unwrap()
                .name
        ))
        .await?;
        return Ok(());
    }

    let mut kill_instruction: Option<String> = instruction;

    if kill_instruction.is_none() {
        kill_instruction = kill_instrucion_missing(ctx).await.unwrap();
    }

    signups
        .as_mut()
        .unwrap()
        .push(Player::new(id, name.unwrap_or(ctx.author().name.clone())));

    instructions
        .as_mut()
        .unwrap()
        .push(Instructions::new(id, kill_instruction.unwrap()));

    ctx.say("You have signed up! Please wait for the game to start.")
        .await?;

    Ok(())
}

async fn kill_instrucion_missing(ctx: BotContext<'_>) -> Result<Option<String>, Error> {
    ctx.say("Please provide a way to kill (next message you send will be used")
        .await?;

    let response: Message = match ctx
        .author()
        .await_reply(&ctx.serenity_context())
        .timeout(std::time::Duration::from_secs(120))
        .await
    {
        Some(response) => response,
        None => {
            ctx.say(format!(
                "{}, You did not provide your information in time.",
                ctx.author().name
            ))
            .await?;
            return Ok(None);
        }
    };

    if response.content.trim().to_string().is_empty() {
        response
            .reply(
                &ctx.http(),
                "Invalid response. Please provide a valid kill instruction.",
            )
            .await?;
    }

    Ok(Some(response.content.trim().to_string()))
}

fn pair_players(players: Vec<Player>, ways_to_kill: Vec<Instructions>) -> Option<Vec<Objective>> {
    if players.len() != ways_to_kill.len() {
        panic!("Vectors must have the same length!");
    } else if players.is_empty() || ways_to_kill.is_empty() {
        return None;
    }
    println!("Players: {:#?}", players.len());

    let mut remaining_players: Vec<Player> = players.clone();
    let mut pairs: Vec<Objective> = Vec::new();
    let mut objectives: Vec<Objective> = Vec::new();
    let mut rng = thread_rng();

    let start_player: Player = remaining_players[0].clone();
    let mut current_player: Player = start_player.clone();
    let mut target: Player;

    for i in 0..players.len() {
        remaining_players.retain(|p| *p != current_player);
        let possible_targets: Vec<Player> = remaining_players.clone();

        if possible_targets.is_empty() {
            target = start_player.clone();
        } else {
            target = possible_targets.choose(&mut rng)?.clone();
        }

        pairs.push(Objective::new(
            Some(i as u64),
            current_player.clone(),
            target.clone(),
            None,
        ));

        current_player = target.clone();
    }

    let objectives_with_instructions = assign_instructions(0, &mut pairs, ways_to_kill.clone());
    if objectives_with_instructions.is_none() {
        panic!("ERROR! Instructions could not be assigned.")
    }
    for objective in objectives_with_instructions.unwrap() {
        objectives.push(objective);
    }

    Some(objectives)
}

fn assign_instructions(
    i: i32,
    objectives: &mut Vec<Objective>,
    remaining_instructions: Vec<Instructions>,
) -> Option<Vec<Objective>> {
    if i == objectives.len() as i32 {
        return Some(objectives.clone());
    }
    let rng = &mut thread_rng();
    let mut possible_instructions: Vec<Instructions> = remaining_instructions
        .clone()
        .into_iter()
        .filter(|p| {
            p.id != objectives[i as usize].player.id && p.id != objectives[i as usize].target.id
        })
        .collect();

    loop {
        if possible_instructions.is_empty() {
            return None;
        }
        let instruction = possible_instructions.choose(rng).unwrap().clone();
        println!(
            "Currently at player {:#?} with target {:#?}, trying {:#?}",
            objectives[i as usize].player.name,
            objectives[i as usize].target.name,
            instruction.instructions
        );
        objectives[i as usize].instructions = Some(instruction.clone());
        let result = assign_instructions(
            i + 1,
            objectives,
            remaining_instructions
                .clone()
                .into_iter()
                .filter(|p| *p != instruction)
                .collect(),
        );
        if result.is_some() {
            return result;
        } else {
            possible_instructions.retain(|p| *p != instruction);
        }
    }
}

fn term(initial_data: Option<(Vec<Player>, Vec<Instructions>)>) {
    if initial_data.is_some() {
        if let Some(result) = pair_players(
            initial_data.clone().unwrap().0,
            initial_data.clone().unwrap().1,
        ) {
            for objective in result {
                println! {"{:#?} -> {:#?}, with instruction {:#?}", objective.clone().player.id, objective.clone().target.id, objective.clone().instructions.unwrap().id};
            }
        } else {
            println! {"Failed to create a valid chain."};
        }
        return;
    }

    println!("Enter the number of players:");

    let mut input_line = String::new();
    io::stdin()
        .read_line(&mut input_line)
        .expect("Failed to read line!");
    let player_number: u64 = match input_line.trim().parse() {
        Ok(num) => num,
        Err(_) => {
            println!("Invalid input. Please enter a number.");
            return;
        }
    };

    if player_number <= 0 {
        println!("Invalid input. Minimum 1 person is required to play.");
        return;
    }

    let mut players: Vec<Player> = Vec::new();
    let mut ways_to_kill: Vec<Instructions> = Vec::new();

    for i in 0..player_number {
        println!("PLAYER {}", i + 1);
        print!("\tName: ");
        io::stdout().flush().expect("Could not flush stdout");
        let mut name = String::new();
        io::stdin()
            .read_line(&mut name)
            .expect("Failed to get name.");
        let player: Player = Player::new(Some(i), name.trim().to_string());
        players.push(player);

        println!("");
    }
    println!("-----------------");
    for i in 0..player_number {
        println!("WAYS TO KILL:");
        print!("\tInstructions #{}: ", i + 1);
        io::stdout().flush().expect("Could not flush stdout");
        let mut instruction = String::new();
        io::stdin()
            .read_line(&mut instruction)
            .expect("Failed to get instructions.");
        let way_to_kill: Instructions = Instructions::new(Some(i), instruction.trim().to_string());
        ways_to_kill.push(way_to_kill);
        println!("");
    }

    if let Some(result) = pair_players(players, ways_to_kill) {
        println! {"{:#?}", result};
    } else {
        println! {"Failed to create a valid chain."};
    }
}

#[poise::command(prefix_command, slash_command)]
async fn help(ctx: BotContext<'_>) -> Result<(), Error> {
    let help_message = "
Welcome to this discord bot! This bot should help assign a target and kill instructions more easily.

Each player should `/sign_up` first, and enter a name and kill instructions. Then, the `admin` can use `/start_game`, and then `/inform`. Once this is done, the game will have started, and will last until the last player uses the `/kill` command.

Good luck and happy hunting!
    ";
    ctx.say(help_message).await?;
    Ok(())
}

/// Manually add a player using id
#[poise::command(prefix_command, slash_command)]
async fn add_player(
    ctx: BotContext<'_>,
    #[description = "User Id"] id: Option<u64>,
    #[description = "Player name"] name: Option<String>,
    #[description = "Kill instruction"] instruction: Option<String>,
) -> Result<(), Error> {
    if !Player::is_admin(ctx).await.unwrap() {
        return Ok(());
    }

    signup(ctx, id, name, instruction).await?;
    Ok(())
}

/// Sign up for the game. Asks for username and kill instruction.
#[poise::command(prefix_command, slash_command)]
async fn sign_up(
    ctx: BotContext<'_>,
    #[description = "Your player name"] name: Option<String>,
    #[description = "Your kill instruction"] instruction: Option<String>,
) -> Result<(), Error> {
    signup(ctx, Some(ctx.author().id.get()), name, instruction).await?;
    Ok(())
}

/// Change the name before starting the game.
#[poise::command(prefix_command, slash_command)]
async fn change_name(
    ctx: BotContext<'_>,
    #[description = "Your new player name"] name: Option<String>,
) -> Result<(), Error> {
    let mut signups = ctx.data().signups.lock().await;
    let mut game_state = ctx.data().game_state.lock().await;

    if States::is_playing(game_state.as_mut().unwrap()) {
        ctx.say("Game already started. Cannot change name anymore.")
            .await?;
        return Ok(());
    } else if *game_state == Some(States::Finished) {
        ctx.say(
            "Game already finished. Cannot change name anymore. Ask the admin to start a new game",
        )
        .await?;
        return Ok(());
    }

    if !signups
        .as_mut()
        .unwrap()
        .iter()
        .any(|c| c.id == ctx.author().id.get())
    {
        ctx.say(format!(
            "You are not signed up yet. Do you wish to sign up? [Y/n]"
        ))
        .await?;

        let response: Message = match ctx
            .author()
            .await_reply(&ctx.serenity_context())
            .timeout(std::time::Duration::from_secs(120))
            .await
        {
            Some(response) => response,
            None => {
                ctx.say(format!(
                    "{}, You did not provide your information in time.",
                    ctx.author().name
                ))
                .await?;
                return Ok(());
            }
        };

        match response.content.trim() {
            "n" | "N" => {
                response.reply(&ctx.http(), "Operation canceled.").await?;
                return Ok(());
            }
            _ => {
                signup(ctx, Some(ctx.author().id.get()), name, None).await?;
                return Ok(());
            }
        };
    }

    if let Some(player) = signups
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p.id == ctx.author().id.get())
    {
        player.name = name.unwrap_or_else(|| ctx.author().name.clone())
    }

    ctx.say("You have changed your name. Please wait for the game to start.")
        .await?;

    Ok(())
}

/// Allows a player to add more instructions
#[poise::command(prefix_command, slash_command)]
async fn add_instruction(
    ctx: BotContext<'_>,
    #[description = "Instructions to add"] instruction: Option<String>,
) -> Result<(), Error> {
    let mut instructions = ctx.data().instructions.lock().await;
    let mut game_state = ctx.data().game_state.lock().await;
    if States::is_playing(game_state.as_mut().unwrap()) {
        ctx.say("Game already started. Cannot change name anymore.")
            .await?;
        return Ok(());
    } else if *game_state == Some(States::Finished) {
        ctx.say(
            "Game already finished. Cannot change name anymore. Ask the admin to start a new game",
        )
        .await?;
        return Ok(());
    }

    if instruction.is_none() {
        ctx.say("No instruction provided!").await?;
    } else {
        instructions.as_mut().unwrap().push(Instructions::new(
            Some(ctx.author().id.get()),
            instruction.unwrap(),
        ));
        ctx.say("Instruction added correctly.").await?;
    }

    Ok(())
}

/// Set the channel where announcements will be sent.
#[poise::command(prefix_command, slash_command)]
async fn set_channel(
    ctx: BotContext<'_>,
    #[description = "Set the channel for the game"] channel: Option<ChannelId>,
) -> Result<(), Error> {
    if !Player::is_admin(ctx).await.unwrap() {
        return Ok(());
    }

    let channel_id = ctx.channel_id();
    let mut data = ctx.data().channel_id.lock().await;
    *data = Some(channel_id);
    if channel.is_none() {
        ctx.say(format!(
            "Channel set to: {:#?} (id: {})",
            channel_id.name(&ctx.http()).await?,
            channel_id.get()
        ))
        .await?;
    } else {
        ctx.say(format!(
            "Channel {} set to: {:#?} (id: {})",
            channel.unwrap(),
            channel_id.name(&ctx.http()).await?,
            channel_id.get()
        ))
        .await?;
    }
    Ok(())
}

/// Get the channel where announcements will be set
#[poise::command(prefix_command, slash_command)]
async fn get_channel(ctx: BotContext<'_>) -> Result<(), Error> {
    let channel_id = ctx.data().channel_id.lock().await;

    if channel_id.is_none() {
        ctx.say("No channel set yet.").await?;
        return Ok(());
    }

    ctx.say(format!(
        "Channel is currently: {:#?} (id: {})",
        channel_id.unwrap().name(&ctx.http()).await?,
        channel_id.unwrap().get()
    ))
    .await?;
    Ok(())
}

/// Starts the game. Used by admins
#[poise::command(prefix_command, slash_command)]
async fn start_game(ctx: BotContext<'_>) -> Result<(), Error> {
    if !Player::is_admin(ctx).await.unwrap() {
        return Ok(());
    }

    let mut game_state = ctx.data().game_state.lock().await;
    let mut signups = ctx.data().signups.lock().await;
    let mut objectives = ctx.data().objectives.lock().await;

    if States::is_playing(game_state.as_mut().unwrap()) {
        ctx.say("A game is already in progress.").await?;
        return Ok(());
    }

    if signups.as_mut().unwrap().len() < 3 {
        ctx.say("Not enough players have signed up. Need at least 3.")
            .await?;
        return Ok(());
    }

    let instructions = ctx.data().instructions.lock().await;

    if let Some(objectives_list) = pair_players(
        signups.as_mut().unwrap().clone(),
        instructions.clone().unwrap(),
    ) {
        *game_state = Some(States::Playing(
            signups.as_mut().unwrap().clone().len() as i32
        ));

        *objectives = Some(objectives_list);

        ctx.say("GAME STARTED").await?;
    } else {
        ctx.say("Failed to create a valid chain. Game not started.")
            .await?;
        *game_state = Some(States::Waiting);
    }

    Ok(())
}

/// Consult target in case you forgot.
#[poise::command(prefix_command, slash_command)]
async fn get_target(ctx: BotContext<'_>) -> Result<(), Error> {
    let game_state = ctx.data().game_state.lock().await;

    let target_objective: Objective = Objective::get_objective(ctx.author().id, ctx).await;

    if *game_state == Some(States::Waiting) {
        not_started_yet(ctx).await?;
        return Ok(());
    }

    ctx.say(format!(
        "Your target is: {}. Your instructions are: {}",
        target_objective.target.name,
        target_objective.instructions.unwrap().instructions
    ))
    .await?;

    Ok(())
}

/// Kill your target. You will recieve a new one
#[poise::command(prefix_command, slash_command)]
async fn kill(ctx: BotContext<'_>) -> Result<(), Error> {
    let mut objectives = ctx.data().objectives.lock().await;
    let mut game_state = ctx.data().game_state.lock().await;

    if *game_state == Some(States::Waiting) {
        not_started_yet(ctx).await?;
        return Ok(());
    }

    if objectives
        .as_mut()
        .unwrap()
        .iter()
        .filter(|p| p.completed != true)
        .count()
        <= 2
    {
        ctx.say("Well done! You won the game!").await?;
        *game_state = Some(States::Finished);
        return Ok(());
    }

    ctx.say("HERE").await?;

    let objective: Objective = Objective::complete(ctx.author().id, ctx).await;

    ctx.say("Well done! You killed the target! Here is your next target:")
        .await?;

    ctx.say(format!(
        "Your target is: {}.\nYour kill instruction is: {}",
        objective.target.name,
        objective.instructions.unwrap().instructions
    ))
    .await?;

    Ok(())
}

/// Admin use. Inform players of their targets
#[poise::command(prefix_command, slash_command)]
async fn inform(ctx: BotContext<'_>) -> Result<(), Error> {
    if !Player::is_admin(ctx).await.unwrap() {
        return Ok(());
    }

    let game_state = ctx.data().game_state.lock().await;
    let mut signups = ctx.data().signups.lock().await;
    let mut objectives = ctx.data().objectives.lock().await;

    if *game_state == Some(States::Waiting) {
        not_started_yet(ctx).await?;
        if signups.is_none() {
            ctx.say("No players have signed up.").await?;
        } else {
            ctx.say(format!(
                "{} players have signed up.",
                signups.as_mut().unwrap().len()
            ))
            .await?;
        }
        return Ok(());
    }

    for objective in objectives
        .as_mut()
        .unwrap()
        .iter()
        .filter(|p| p.player.id > 10)
    {
        let message = CreateMessage::new();
        let user: UserId = UserId::new(objective.player.id); // (&ctx.http()).await?;
        user.dm(
            &ctx.http(),
            message.content(
                format!(
                    "Hello {}, your target is {}, and your kill instruction is: {} (use /kill to eliminate target)",
                    objective.player.name,
                    objective.target.name,
                    objective.instructions.as_ref().unwrap().instructions
                )
                .clone(),
            ),
        )
        .await?;
    }

    ctx.say("All players have been informed of their kill instructions privately.")
        .await?;
    Ok(())
}

/// Admin use. Reveal all information from signup and targets
#[poise::command(prefix_command, slash_command)]
async fn reveal(ctx: BotContext<'_>) -> Result<(), Error> {
    if !Player::is_admin(ctx).await.unwrap() {
        return Ok(());
    }

    let mut objectives = ctx.data().objectives.lock().await;
    let mut signups = ctx.data().signups.lock().await;
    let mut instructions = ctx.data().instructions.lock().await;
    let mut game_state = ctx.data().game_state.lock().await;

    match (*game_state).as_mut().unwrap() {
        States::Waiting => {
            ctx.say("The game has not started yet.").await?;
        }
        States::Playing(_) => {
            ctx.say("The game is already in progress.").await?;
        }
        States::Finished => {
            ctx.say("The game has already finished.").await?;
        }
    }

    if signups.as_mut().unwrap().is_empty() {
        ctx.say("No players have signed up.").await?;
        return Ok(());
    }

    let mut message: String = Default::default();

    for instruction in instructions.as_mut().unwrap() {
        message.push_str(&format!(
            "{} wrote the instruction {:#?}\n",
            signups
                .as_mut()
                .unwrap_or(&mut vec![])
                .iter()
                .find(|signup| signup.id == instruction.id)
                .unwrap()
                .name,
            instruction.instructions
        ));
    }

    ctx.say(message).await?;

    for objective in objectives.as_mut().unwrap_or(&mut vec![]) {
        ctx.say(format!(
            "{}'s target is {} and their kill instruction is: {}",
            objective.player.name,
            objective.target.name,
            objective.instructions.as_ref().unwrap().instructions
        ))
        .await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        println!("Running in the terminal.");
        term(None);
        return;
    }

    // Load the token from the Secrets.toml file
    let mut file_content = String::new();
    let mut file = fs::File::open("Secrets.toml").expect("Could not open Secrets.toml");
    file.read_to_string(&mut file_content)
        .expect("Could not read Secrets.toml");

    let secrets: Secrets = toml::from_str(&file_content).expect("Could not parse Secrets.toml");
    let token = secrets.discord_token;

    let intents = //serenity::GatewayIntents::GUILD_MEMBERS
        serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::DIRECT_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                help(),
                add_player(),
                sign_up(),
                change_name(),
                add_instruction(),
                get_target(),
                set_channel(),
                get_channel(),
                start_game(),
                kill(),
                inform(),
                reveal(),
            ],
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(GameData {
                    admin_id: Arc::new(Mutex::new(None)),
                    game_state: Arc::new(Mutex::new(Some(States::Waiting))),
                    channel_id: Arc::new(Mutex::new(None)),
                    signups: Arc::new(Mutex::new(Some(vec![]))),
                    instructions: Arc::new(Mutex::new(Some(vec![]))),
                    objectives: Arc::new(Mutex::new(None)),
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
