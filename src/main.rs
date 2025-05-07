use chrono::Local;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Input, Select, MultiSelect};
use dotenv::dotenv;
use indicatif::{ProgressBar, ProgressStyle};
use rig::{
    completion::{Chat, Message, AssistantContent},
    providers::openai,
    OneOrMany,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fs, path::Path, thread, time::Duration, io};

const SAVE_FILE: &str = "dnd_adventure_save.json";

const TITLE_ART: &str = r#"
  _____                                                 _____                             
 |  __ \                                               |  __ \                            
 | |  | |_   _ _ __   __ _  ___  ___  _ __  ___       | |  | |_ __ __ _  __ _  ___  _ __  ___  
 | |  | | | | | '_ \ / _` |/ _ \/ _ \| '_ \/ __|      | |  | | '__/ _` |/ _` |/ _ \| '_ \/ __| 
 | |__| | |_| | | | | (_| |  __/ (_) | | | \__ \      | |__| | | | (_| | (_| | (_) | | | \__ \ 
 |_____/ \__,_|_| |_|\__, |\___|\___/|_| |_|___/      |_____/|_|  \__,_|\__, |\___/|_| |_|___/ 
                      __/ |                                               __/ |                 
                     |___/                                               |___/                  
"#;

// Character data structures
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Character {
    name: String,
    race: String,
    class: String,
    level: u32,
    strength: u32,
    dexterity: u32,
    constitution: u32,
    intelligence: u32,
    wisdom: u32,
    charisma: u32,
    hit_points: u32,
    max_hit_points: u32,
    armor_class: u32,
    inventory: Vec<String>,
    gold: u32,
    experience: u32,
    background: String,
    skills: HashMap<String, bool>,
}

impl Default for Character {
    fn default() -> Self {
        let mut skills = HashMap::new();
        for skill in &[
            "Acrobatics", "Animal Handling", "Arcana", "Athletics", "Deception", 
            "History", "Insight", "Intimidation", "Investigation", "Medicine", 
            "Nature", "Perception", "Performance", "Persuasion", "Religion", 
            "Sleight of Hand", "Stealth", "Survival"
        ] {
            skills.insert(skill.to_string(), false);
        }
        
        Self {
            name: String::new(),
            race: String::new(),
            class: String::new(),
            level: 1,
            strength: 10,
            dexterity: 10,
            constitution: 10,
            intelligence: 10,
            wisdom: 10,
            charisma: 10,
            hit_points: 10,
            max_hit_points: 10,
            armor_class: 10,
            inventory: Vec::new(),
            gold: 0,
            experience: 0,
            background: String::new(),
            skills,
        }
    }
}

// Game state
#[derive(Serialize, Deserialize, Clone, Debug)]
struct GameState {
    character: Character,
    campaign: String,
    current_location: String,
    current_quest: String,
    history: Vec<Message>,
    date_started: String,
    last_saved: String,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            character: Character::default(),
            campaign: String::new(),
            current_location: String::new(),
            current_quest: String::new(),
            history: Vec::new(),
            date_started: Local::now().to_rfc3339(),
            last_saved: Local::now().to_rfc3339(),
        }
    }
}

// Display utilities
fn show_spinner(message: &str, duration_ms: u64) {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    
    for _ in 0..duration_ms / 100 {
        pb.tick();
        thread::sleep(Duration::from_millis(100));
    }
    
    pb.finish_and_clear();
}

fn print_header() {
    let title = TITLE_ART.bright_cyan().bold();
    println!("\n{}", title);
    println!("{}", "AI Dungeon Master".bright_purple().bold());
    println!("{}\n", "=".repeat(60).bright_blue());
}

fn print_fancy_message(message: &str, color: &str) {
    let formatted = match color {
        "red" => message.bright_red().bold(),
        "green" => message.bright_green().bold(),
        "blue" => message.bright_blue().bold(),
        "yellow" => message.bright_yellow().bold(),
        "cyan" => message.bright_cyan().bold(),
        "magenta" => message.bright_magenta().bold(),
        "white" => message.bright_white().bold(),
        _ => message.normal(),
    };
    
    println!("\n{}", formatted);
}

fn print_character_sheet(character: &Character) {
    println!("\n{}", "CHARACTER SHEET".bright_yellow().bold());
    println!("{}", "=".repeat(60).bright_blue());
    println!("{}: {}", "Name".bright_green(), character.name.bright_white());
    println!("{}: {} | {}: {}", 
             "Race".bright_green(), character.race.bright_white(),
             "Class".bright_green(), character.class.bright_white());
    println!("{}: {} | {}: {} GP", 
             "Level".bright_green(), character.level.to_string().bright_white(),
             "Gold".bright_green(), character.gold.to_string().bright_white());
    println!("{}", "-".repeat(60).bright_blue());
    println!("{}: {}/{}", 
             "Hit Points".bright_green(), 
             character.hit_points.to_string().bright_white(),
             character.max_hit_points.to_string().bright_white());
    println!("{}: {}", 
             "Armor Class".bright_green(), 
             character.armor_class.to_string().bright_white());
    println!("{}", "-".repeat(60).bright_blue());
    println!("{}", "Abilities".bright_yellow());
    println!("{}: {} | {}: {}",
             "STR".bright_green(), character.strength.to_string().bright_white(),
             "DEX".bright_green(), character.dexterity.to_string().bright_white());
    println!("{}: {} | {}: {}",
             "CON".bright_green(), character.constitution.to_string().bright_white(),
             "INT".bright_green(), character.intelligence.to_string().bright_white());
    println!("{}: {} | {}: {}",
             "WIS".bright_green(), character.wisdom.to_string().bright_white(),
             "CHA".bright_green(), character.charisma.to_string().bright_white());
    println!("{}", "-".repeat(60).bright_blue());
    
    println!("{}", "Inventory".bright_yellow());
    if character.inventory.is_empty() {
        println!("(empty)");
    } else {
        for item in &character.inventory {
            println!("• {}", item);
        }
    }
    println!("{}", "=".repeat(60).bright_blue());
}

// File operations
fn save_game(state: &GameState) -> Result<(), Box<dyn Error>> {
    // Create a temporary file to write to first
    let temp_file = format!("{}.tmp", SAVE_FILE);
    let json = serde_json::to_string_pretty(state)?;
    
    // Write to the temporary file first
    fs::write(&temp_file, &json)?;
    
    // Then rename the temporary file to the actual save file
    // This helps prevent corruption if the program crashes during the write
    if Path::new(&temp_file).exists() {
        if Path::new(SAVE_FILE).exists() {
            fs::remove_file(SAVE_FILE)?;
        }
        fs::rename(&temp_file, SAVE_FILE)?;
    }
    
    Ok(())
}

fn load_game() -> Result<GameState, Box<dyn Error>> {
    if Path::new(SAVE_FILE).exists() {
        let json = fs::read_to_string(SAVE_FILE)?;
        let state: GameState = serde_json::from_str(&json)?;
        Ok(state)
    } else {
        Ok(GameState::default())
    }
}

// Helper function to extract text from OneOrMany<AssistantContent>
fn extract_text_from_message(content: &OneOrMany<AssistantContent>) -> String {
    // Try to extract the text from the first element using the public API
    let text = match content.first() {
        AssistantContent::Text(text_content) => text_content.text.clone(),
        _ => "No readable text content available".to_string(),
    };
    
    text
}

// Dice rolling utilities
fn roll_dice(num_dice: u32, sides: u32) -> Vec<u32> {
    let mut rng = rand::thread_rng();
    let mut results = Vec::new();
    
    for _ in 0..num_dice {
        results.push(rng.gen_range(1..=sides));
    }
    
    results
}

fn print_dice_roll(dice_type: &str, results: &[u32]) {
    let sum: u32 = results.iter().sum();
    let dice_results = results
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    
    println!("{} {} [{}] = {}", 
             "Rolled".bright_blue(),
             dice_type.bright_yellow(),
             dice_results.bright_white(),
             sum.to_string().bright_green().bold());
}

// AI DM interactions
async fn dm_chat<C>(
    dm: &C,
    prompt: &str,
    history: Vec<Message>,
    error_message: &str,
    spinner_message: &str,
    spinner_duration: u64,
) -> Result<String, Box<dyn Error>>
where
    C: Chat,
{
    show_spinner(spinner_message, spinner_duration);
    
    match dm.chat(prompt, history).await {
        Ok(response) => Ok(response),
        Err(e) => {
            print_fancy_message("The Dungeon Master cannot respond...", "red");
            println!("Error: {}", e);
            Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                error_message,
            )))
        }
    }
}

async fn start_new_campaign(
    dm: &impl Chat,
    character: Character,
) -> Result<GameState, Box<dyn Error>> {
    let mut state = GameState {
        character,
        date_started: Local::now().to_rfc3339(),
        last_saved: Local::now().to_rfc3339(),
        ..Default::default()
    };
    
    // Create campaign prompt
    let campaign_prompt = format!(
        "You are the Dungeon Master for a Dungeons & Dragons 5e adventure. 
        Create an exciting campaign hook and starting location for a {} {} named {}. 
        The character is level {} with the following stats: 
        STR {}, DEX {}, CON {}, INT {}, WIS {}, CHA {}.
        Background: {}.
        
        Provide a brief introduction to the campaign setting, including:
        1. The name of the campaign/adventure
        2. The starting location (town/city/village name)
        3. The initial quest or hook to draw the player in
        4. A brief description of the area and its people
        
        Format your response with these section headers but focus on immersive, evocative descriptions rather than mechancial details. Make it engaging and atmospheric!",
        state.character.race,
        state.character.class,
        state.character.name,
        state.character.level,
        state.character.strength,
        state.character.dexterity,
        state.character.constitution,
        state.character.intelligence,
        state.character.wisdom,
        state.character.charisma,
        state.character.background
    );
    
    let campaign_response = dm_chat(
        dm,
        &campaign_prompt,
        vec![],
        "Failed to communicate with the Dungeon Master",
        "The Dungeon Master is creating your adventure...",
        5000,
    )
    .await?;
    
    // Parse the response for campaign details
    for line in campaign_response.lines() {
        if line.to_lowercase().contains("campaign") || line.to_lowercase().contains("adventure") {
            if let Some(name) = line.split(":").nth(1) {
                state.campaign = name.trim().to_string();
            }
        }
        if line.to_lowercase().contains("location") {
            if let Some(location) = line.split(":").nth(1) {
                state.current_location = location.trim().to_string();
            }
        }
        if line.to_lowercase().contains("quest") || line.to_lowercase().contains("hook") {
            if let Some(quest) = line.split(":").nth(1) {
                state.current_quest = quest.trim().to_string();
            }
        }
    }
    
    // If we couldn't parse the details, use default values
    if state.campaign.is_empty() {
        state.campaign = "Mystical Adventure".to_string();
    }
    if state.current_location.is_empty() {
        state.current_location = "Starting Town".to_string();
    }
    if state.current_quest.is_empty() {
        state.current_quest = "Find adventure".to_string();
    }
    
    // Add to history
    state.history.push(Message::user(&campaign_prompt));
    state.history.push(Message::assistant(&campaign_response));
    
    // Add a scene-setting message
    let scene_setting = "Now, describe the opening scene. The player's character has just arrived at the starting location. Provide rich sensory details and introduce an NPC or situation that connects to the quest hook. End with a question or prompt for the player to respond to.";
    
    let scene_response = dm_chat(
        dm,
        scene_setting,
        state.history.clone(),
        "Failed to set the scene",
        "The Dungeon Master is setting the scene...",
        2500,
    )
    .await?;
    
    state.history.push(Message::user(scene_setting));
    state.history.push(Message::assistant(&scene_response));
    
    save_game(&state)?;
    
    Ok(state)
}

async fn process_player_action(
    dm: &impl Chat,
    action: &str,
    state: &mut GameState,
) -> Result<String, Box<dyn Error>> {
    // Construct the action prompt
    let action_prompt = format!(
        "The player ({} the {} {}) takes the following action:\n\n{}
        
        Respond as the Dungeon Master, describing the outcome of this action. 
        Use rich, evocative language to create an immersive experience.
        If dice rolls would be needed, describe the check but don't roll dice yourself.
        End with either a question or a prompt that gives the player clear options for what they might do next.
        If the player attempts something impossible, gently steer them toward better options.",
        state.character.name,
        state.character.race,
        state.character.class,
        action
    );
    
    let response = dm_chat(
        dm,
        &action_prompt,
        state.history.clone(),
        "Failed to process your action",
        "The Dungeon Master is responding...",
        2000,
    )
    .await?;
    
    state.history.push(Message::user(&action_prompt));
    state.history.push(Message::assistant(&response));
    
    state.last_saved = Local::now().to_rfc3339();
    save_game(state)?;
    
    Ok(response)
}

async fn roll_skill_check(
    dm: &impl Chat,
    skill: &str,
    roll_result: u32,
    state: &mut GameState,
) -> Result<String, Box<dyn Error>> {
    // Get the appropriate ability modifier based on the skill
    let ability_mod = match skill {
        "Athletics" => (state.character.strength as i32 - 10) / 2,
        "Acrobatics" | "Sleight of Hand" | "Stealth" => (state.character.dexterity as i32 - 10) / 2,
        "Arcana" | "History" | "Investigation" | "Nature" | "Religion" => (state.character.intelligence as i32 - 10) / 2,
        "Animal Handling" | "Insight" | "Medicine" | "Perception" | "Survival" => (state.character.wisdom as i32 - 10) / 2,
        "Deception" | "Intimidation" | "Performance" | "Persuasion" => (state.character.charisma as i32 - 10) / 2,
        _ => 0,
    };
    
    // Apply proficiency bonus if proficient
    let prof_bonus = match state.character.level {
        1..=4 => 2,
        5..=8 => 3,
        9..=12 => 4,
        13..=16 => 5,
        _ => 6,
    };
    
    let is_proficient = state.character.skills.get(skill).unwrap_or(&false);
    let total = roll_result as i32 + ability_mod + if *is_proficient { prof_bonus } else { 0 };
    
    let roll_prompt = format!(
        "The player ({} the {} {}) rolls a {} check.
        Dice roll: {}
        Ability modifier: {}
        Proficiency: {}
        Total: {}
        
        Interpret this skill check result in the current context. 
        Describe the outcome of their action based on this result.
        For reference, typical difficulty classes are:
        - Easy: 10
        - Medium: 15
        - Hard: 20
        - Very Hard: 25
        - Nearly Impossible: 30
        
        Continue the scene after describing the result of this check.",
        state.character.name,
        state.character.race,
        state.character.class,
        skill,
        roll_result,
        ability_mod,
        if *is_proficient { "Yes (+2)" } else { "No" },
        total
    );
    
    let response = dm_chat(
        dm,
        &roll_prompt,
        state.history.clone(),
        "Failed to process skill check",
        "The Dungeon Master is resolving your check...",
        2000,
    )
    .await?;
    
    state.history.push(Message::user(&roll_prompt));
    state.history.push(Message::assistant(&response));
    
    state.last_saved = Local::now().to_rfc3339();
    save_game(state)?;
    
    Ok(response)
}

// Character creation functions
fn create_character() -> Character {
    let mut character = Character::default();
    
    print_fancy_message("CHARACTER CREATION", "cyan");
    println!("{}", "Let's create your D&D character!".bright_white());
    
    // Get character name
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is your character's name?")
        .interact_text()
        .unwrap_or_else(|_| "Adventurer".to_string());
    character.name = name;
    
    // Choose race
    let races = vec![
        "Human", "Elf", "Dwarf", "Halfling", "Gnome",
        "Half-Elf", "Half-Orc", "Tiefling", "Dragonborn"
    ];
    
    println!("\n{}", "Choose your race:".bright_yellow());
    let race_index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Race")
        .default(0)
        .items(&races)
        .interact()
        .unwrap_or(0);
    
    character.race = races[race_index].to_string();
    
    // Choose class
    let classes = vec![
        "Fighter", "Wizard", "Cleric", "Rogue", "Ranger",
        "Paladin", "Barbarian", "Bard", "Druid", "Monk",
        "Sorcerer", "Warlock", "Artificer"
    ];
    
    println!("\n{}", "Choose your class:".bright_yellow());
    let class_index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Class")
        .default(0)
        .items(&classes)
        .interact()
        .unwrap_or(0);
    
    character.class = classes[class_index].to_string();
    
    // Choose background
    let backgrounds = vec![
        "Acolyte", "Charlatan", "Criminal", "Entertainer", "Folk Hero",
        "Guild Artisan", "Hermit", "Noble", "Outlander", "Sage",
        "Sailor", "Soldier", "Urchin"
    ];
    
    println!("\n{}", "Choose your background:".bright_yellow());
    let bg_index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Background")
        .default(0)
        .items(&backgrounds)
        .interact()
        .unwrap_or(0);
    
    character.background = backgrounds[bg_index].to_string();
    
    // Roll or assign ability scores
    println!("\n{}", "How would you like to determine your ability scores?".bright_yellow());
    let score_methods = vec!["Roll 4d6 (drop lowest)", "Standard Array", "Point Buy"];
    let score_method = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Method")
        .default(0)
        .items(&score_methods)
        .interact()
        .unwrap_or(0);
    
    let mut scores = Vec::new();
    
    match score_method {
        0 => {
            // Roll 4d6 drop lowest
            println!("\n{}", "Rolling ability scores (4d6 drop lowest)...".bright_blue());
            for i in 0..6 {
                let mut roll = roll_dice(4, 6);
                roll.sort();
                roll.remove(0); // Remove lowest die
                let score: u32 = roll.iter().sum();
                println!("Roll {}: {} = {}", i + 1, 
                         roll.iter().map(|d| d.to_string()).collect::<Vec<String>>().join(", "),
                         score);
                scores.push(score);
            }
        },
        1 => {
            // Standard Array
            println!("\n{}", "Using Standard Array: 15, 14, 13, 12, 10, 8".bright_blue());
            scores = vec![15, 14, 13, 12, 10, 8];
        },
        _ => {
            // Point Buy (simplified)
            println!("\n{}", "Using Point Buy (27 points)".bright_blue());
            scores = vec![13, 13, 13, 12, 12, 8];
        }
    }
    
    // Assign ability scores
    let abilities = vec!["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"];
    let mut assigned_scores = HashMap::new();
    
    println!("\n{}", "Assign your ability scores:".bright_yellow());
    for ability in &abilities {
        let available_scores: Vec<String> = scores
            .iter()
            .map(|s| s.to_string())
            .collect();
        
        println!("\nAvailable scores: {}", available_scores.join(", "));
        let score_index = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Choose a score for {}", ability))
            .items(&available_scores)
            .default(0)
            .interact()
            .unwrap_or(0);
        
        assigned_scores.insert(ability.to_string(), scores[score_index]);
        scores.remove(score_index);
    }
    
    character.strength = *assigned_scores.get("Strength").unwrap_or(&10);
    character.dexterity = *assigned_scores.get("Dexterity").unwrap_or(&10);
    character.constitution = *assigned_scores.get("Constitution").unwrap_or(&10);
    character.intelligence = *assigned_scores.get("Intelligence").unwrap_or(&10);
    character.wisdom = *assigned_scores.get("Wisdom").unwrap_or(&10);
    character.charisma = *assigned_scores.get("Charisma").unwrap_or(&10);
    
    // Calculate hit points based on class and constitution
    let con_modifier = (character.constitution as i32 - 10) / 2;
    let base_hp = match character.class.as_str() {
        "Barbarian" => 12,
        "Fighter" | "Paladin" | "Ranger" => 10,
        "Cleric" | "Druid" | "Monk" => 8,
        "Bard" | "Rogue" | "Warlock" => 8,
        "Sorcerer" | "Wizard" => 6,
        _ => 8,
    };
    
    character.hit_points = (base_hp as i32 + con_modifier).max(1) as u32;
    character.max_hit_points = character.hit_points;
    
    // Set armor class based on dexterity
    let dex_modifier = (character.dexterity as i32 - 10) / 2;
    character.armor_class = (10 + dex_modifier).max(1) as u32;
    
    // Choose skill proficiencies
    println!("\n{}", "Choose skill proficiencies:".bright_yellow());
    
    // How many skills they can choose
    let num_skills = match character.class.as_str() {
        "Rogue" => 4,
        "Bard" | "Ranger" => 3,
        _ => 2,
    };
    
    println!("Your class ({}) lets you choose {} skill proficiencies:", 
             character.class.bright_green(), num_skills.to_string().bright_green());
    
    // Filter available skills based on class
    let available_skills: Vec<&str> = match character.class.as_str() {
        "Barbarian" => vec!["Animal Handling", "Athletics", "Intimidation", "Nature", "Perception", "Survival"],
        "Bard" => vec!["Acrobatics", "Animal Handling", "Arcana", "Athletics", "Deception", 
                        "History", "Insight", "Intimidation", "Investigation", "Medicine", 
                        "Nature", "Perception", "Performance", "Persuasion", "Religion", 
                        "Sleight of Hand", "Stealth", "Survival"],
        "Cleric" => vec!["History", "Insight", "Medicine", "Persuasion", "Religion"],
        "Druid" => vec!["Arcana", "Animal Handling", "Insight", "Medicine", "Nature", "Perception", "Religion", "Survival"],
        "Fighter" => vec!["Acrobatics", "Animal Handling", "Athletics", "History", "Insight", "Intimidation", "Perception", "Survival"],
        "Monk" => vec!["Acrobatics", "Athletics", "History", "Insight", "Religion", "Stealth"],
        "Paladin" => vec!["Athletics", "Insight", "Intimidation", "Medicine", "Persuasion", "Religion"],
        "Ranger" => vec!["Animal Handling", "Athletics", "Insight", "Investigation", "Nature", "Perception", "Stealth", "Survival"],
        "Rogue" => vec!["Acrobatics", "Athletics", "Deception", "Insight", "Intimidation", "Investigation", "Perception", "Performance", "Persuasion", "Sleight of Hand", "Stealth"],
        "Sorcerer" => vec!["Arcana", "Deception", "Insight", "Intimidation", "Persuasion", "Religion"],
        "Warlock" => vec!["Arcana", "Deception", "History", "Intimidation", "Investigation", "Nature", "Religion"],
        "Wizard" => vec!["Arcana", "History", "Insight", "Investigation", "Medicine", "Religion"],
        _ => vec!["Arcana", "History", "Investigation", "Nature", "Religion"],
    };
    
    // Safety check - ensure num_skills doesn't exceed available skills
    let max_selectable = std::cmp::min(num_skills as usize, available_skills.len());
    
    // Use a safer approach with MultiSelect
    let skill_selections = if !available_skills.is_empty() {
        // Create a string explaining selection
        let prompt = format!("Select {} skills (space to select, enter to confirm)", max_selectable);
        
        // Store the theme in a variable to extend its lifetime
        let theme = ColorfulTheme::default();
        
        // Use MultiSelect directly
        MultiSelect::with_theme(&theme)
            .with_prompt(prompt)
            .items(&available_skills)
            .interact()
            .unwrap_or_else(|_| vec![])
    } else {
        vec![]
    };
    
    // Apply the selections safely
    for &index in &skill_selections {
        if index < available_skills.len() {
            let skill = available_skills[index];
            character.skills.insert(skill.to_string(), true);
        }
    }
    
    // Starting equipment based on class
    match character.class.as_str() {
        "Fighter" => {
            character.inventory.push("Longsword".to_string());
            character.inventory.push("Shield".to_string());
            character.inventory.push("Chain mail".to_string());
            character.inventory.push("Dungeoneer's pack".to_string());
            character.armor_class = 16; // Chain mail
            character.gold = 10;
        },
        "Wizard" => {
            character.inventory.push("Spellbook".to_string());
            character.inventory.push("Staff".to_string());
            character.inventory.push("Component pouch".to_string());
            character.inventory.push("Scholar's pack".to_string());
            character.gold = 25;
        },
        "Cleric" => {
            character.inventory.push("Mace".to_string());
            character.inventory.push("Scale mail".to_string());
            character.inventory.push("Shield".to_string());
            character.inventory.push("Holy symbol".to_string());
            character.armor_class = 14 + (dex_modifier.min(2)) as u32; // Scale mail
            character.gold = 15;
        },
        "Rogue" => {
            character.inventory.push("Shortsword".to_string());
            character.inventory.push("Shortbow with 20 arrows".to_string());
            character.inventory.push("Leather armor".to_string());
            character.inventory.push("Thieves' tools".to_string());
            character.armor_class = 11 + dex_modifier as u32; // Leather armor
            character.gold = 30;
        },
        _ => {
            character.inventory.push("Adventurer's pack".to_string());
            character.inventory.push("Simple weapon".to_string());
            character.gold = 20;
        }
    };
    
    // Add common items
    character.inventory.push("Backpack".to_string());
    character.inventory.push("Bedroll".to_string());
    character.inventory.push("Rations (5 days)".to_string());
    character.inventory.push("Waterskin".to_string());
    character.inventory.push("Torch (3)".to_string());
    
    print_fancy_message("Character Created Successfully!", "green");
    print_character_sheet(&character);
    
    character
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = dotenv().ok();
    let openai = openai::Client::from_env();
    
    let dungeon_master = openai
        .agent("gpt-4.1")
        .preamble(
            "You are an expert Dungeon Master for a Dungeons & Dragons 5th Edition game. 
            
            Your role is to create an immersive, engaging, and dynamic D&D experience in a text-based format. You will:
            
            1. Create rich, evocative descriptions of locations, NPCs, monsters, and scenarios
            2. Respond to player actions by narrating outcomes and advancing the story
            3. Incorporate D&D rules when appropriate, but prioritize storytelling over strict rule adherence
            4. Craft a compelling narrative that responds to player choices
            5. Present interesting challenges, puzzles, and combat encounters
            6. Maintain consistent world details and NPC personalities
            
            Important guidelines:
            - Use vivid, sensory language to create immersion
            - Keep descriptions concise but evocative
            - Present clear options for the player but allow creative actions
            - Balance combat, exploration, and social interaction
            - Adapt the story based on player choices
            - Include elements of mystery and discovery
            - Create memorable NPCs with distinct personalities
            
            Always respond in character as the Dungeon Master and make the adventure feel like a real D&D session. Present options in an open-ended way that encourages player agency and creativity."
        )
        .temperature(0.7)
        .build();
    
    // Main game loop
    loop {
        print_header();
        
        let selections = vec!["Start New Adventure", "Continue Saved Adventure", "View Rules & Commands", "Quit"];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose an option:")
            .default(0)
            .items(&selections)
            .interact()?;
        
        match selection {
            0 => {
                // Start New Adventure
                print_fancy_message("Starting a new adventure...", "cyan");
                
                // Create a character
                let character = create_character();
                
                // Start the campaign with the new character
                let mut state = start_new_campaign(&dungeon_master, character).await?;
                
                print_fancy_message(&format!("Welcome to {}", state.campaign), "yellow");
                
                // Extract the last AI message to show to the player
                if let Some(message) = state.history.last() {
                    if let Message::Assistant { content } = message {
                        // Extract and display just the text content from OneOrMany
                        // Extract the text from the message content
                        let text = extract_text_from_message(content);
                        println!("{}", text.bright_white());
                    }
                }
                
                // Adventure gameplay loop
                loop {
                    println!("\n{}", "-".repeat(60).bright_blue());
                    println!("{}: {} | {}: {}", 
                             "Location".bright_green(), state.current_location.bright_white(),
                             "Quest".bright_green(), state.current_quest.bright_white());
                    println!("{}: {}/{} HP | {}: {} AC", 
                             state.character.name.bright_yellow(),
                             state.character.hit_points.to_string().bright_white(),
                             state.character.max_hit_points.to_string().bright_white(),
                             "AC".bright_yellow(),
                             state.character.armor_class.to_string().bright_white());
                    println!("{}", "-".repeat(60).bright_blue());
                    
                    // Show player options
                    println!("\n{}", "What would you like to do?".bright_cyan());
                    let actions = vec![
                        "Take an action", 
                        "Roll a skill check", 
                        "Roll a dice", 
                        "Show character sheet",
                        "Save game",
                        "Return to main menu"
                    ];
                    
                    let action_choice = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Choose an action")
                        .default(0)
                        .items(&actions)
                        .interact()?;
                    
                    match action_choice {
                        0 => {
                            // Take an action
                            let player_action: String = Input::with_theme(&ColorfulTheme::default())
                                .with_prompt("What would you like to do? (describe your action)")
                                .interact_text()?;
                            
                            let dm_response = process_player_action(&dungeon_master, &player_action, &mut state).await?;
                            print_fancy_message("Dungeon Master:", "cyan");
                            println!("{}", dm_response.bright_white());
                        },
                        1 => {
                            // Roll a skill check
                            let skills = vec![
                                "Acrobatics", "Animal Handling", "Arcana", "Athletics", "Deception", 
                                "History", "Insight", "Intimidation", "Investigation", "Medicine", 
                                "Nature", "Perception", "Performance", "Persuasion", "Religion", 
                                "Sleight of Hand", "Stealth", "Survival"
                            ];
                            
                            let skill_index = Select::with_theme(&ColorfulTheme::default())
                                .with_prompt("Choose a skill to check")
                                .default(0)
                                .items(&skills)
                                .interact()?;
                            
                            let skill = skills[skill_index];
                            let is_proficient = state.character.skills.get(skill).unwrap_or(&false);
                            
                            // Roll the d20
                            let d20_results = roll_dice(1, 20);
                            let roll_result = d20_results[0];
                            
                            // Print the roll
                            print_fancy_message(&format!("{} Check", skill), "yellow");
                            print_dice_roll("d20", &d20_results);
                            
                            // Get ability modifier
                            let ability_mod = match skill {
                                "Athletics" => (state.character.strength as i32 - 10) / 2,
                                "Acrobatics" | "Sleight of Hand" | "Stealth" => (state.character.dexterity as i32 - 10) / 2,
                                "Arcana" | "History" | "Investigation" | "Nature" | "Religion" => (state.character.intelligence as i32 - 10) / 2,
                                "Animal Handling" | "Insight" | "Medicine" | "Perception" | "Survival" => (state.character.wisdom as i32 - 10) / 2,
                                "Deception" | "Intimidation" | "Performance" | "Persuasion" => (state.character.charisma as i32 - 10) / 2,
                                _ => 0,
                            };
                            
                            // Calculate proficiency bonus
                            let prof_bonus = match state.character.level {
                                1..=4 => 2,
                                5..=8 => 3,
                                9..=12 => 4,
                                13..=16 => 5,
                                _ => 6,
                            };
                            
                            // Calculate total
                            let total = roll_result as i32 + ability_mod + if *is_proficient { prof_bonus } else { 0 };
                            
                            println!("Ability modifier: {}", ability_mod);
                            if *is_proficient {
                                println!("Proficiency bonus: +{}", prof_bonus);
                            }
                            println!("Total: {}", total.to_string().bright_green().bold());
                            
                            // Process the skill check with the DM
                            let dm_response = roll_skill_check(&dungeon_master, skill, roll_result, &mut state).await?;
                            print_fancy_message("Dungeon Master:", "cyan");
                            println!("{}", dm_response.bright_white());
                        },
                        2 => {
                            // Roll dice
                            let dice_types = vec!["d4", "d6", "d8", "d10", "d12", "d20", "d100"];
                            let dice_type_index = Select::with_theme(&ColorfulTheme::default())
                                .with_prompt("Choose a dice type")
                                .default(5) // d20 is default
                                .items(&dice_types)
                                .interact()?;
                            
                            let sides = match dice_types[dice_type_index] {
                                "d4" => 4,
                                "d6" => 6,
                                "d8" => 8,
                                "d10" => 10,
                                "d12" => 12,
                                "d20" => 20,
                                "d100" => 100,
                                _ => 6,
                            };
                            
                            let num_dice: String = Input::with_theme(&ColorfulTheme::default())
                                .with_prompt("How many dice?")
                                .default("1".to_string())
                                .interact_text()?;
                            
                            let num_dice = num_dice.parse::<u32>().unwrap_or(1);
                            let results = roll_dice(num_dice, sides);
                            
                            print_fancy_message("Dice Roll", "yellow");
                            print_dice_roll(&format!("{}d{}", num_dice, sides), &results);
                        },
                        3 => {
                            // Show character sheet
                            print_character_sheet(&state.character);
                        },
                        4 => {
                            // Save game
                            match save_game(&state) {
                                Ok(_) => print_fancy_message("Game saved successfully!", "green"),
                                Err(e) => print_fancy_message(&format!("Error saving game: {}", e), "red"),
                            }
                        },
                        5 => {
                            // Return to main menu
                            print_fancy_message("Returning to main menu...", "blue");
                            break;
                        },
                        _ => unreachable!(),
                    }
                }
            },
            1 => {
                // Continue Saved Adventure
                match load_game() {
                    Ok(mut state) => {
                        if state.campaign.is_empty() {
                            print_fancy_message("No saved adventure found!", "red");
                            thread::sleep(Duration::from_secs(2));
                            continue;
                        }
                        
                        print_fancy_message(&format!("Continuing your adventure in {}...", state.campaign), "blue");
                        println!("Location: {} | Quest: {}", 
                                 state.current_location.yellow(),
                                 state.current_quest.yellow());
                        
                        // Extract the last AI message to show to the player
                        if let Some(message) = state.history.last() {
                            if let Message::Assistant { content } = message {
                                print_fancy_message("Previously in your adventure:", "cyan");
                                // Extract and display just the text content from OneOrMany
                                // Extract the text from the message content
                                let text = extract_text_from_message(content);
                                println!("{}", text.bright_white());
                            }
                        }
                        
                        // Continue adventure gameplay loop
                        loop {
                            println!("\n{}", "-".repeat(60).bright_blue());
                            println!("{}: {} | {}: {}", 
                                     "Location".bright_green(), state.current_location.bright_white(),
                                     "Quest".bright_green(), state.current_quest.bright_white());
                            println!("{}: {}/{} HP | {}: {} AC", 
                                     state.character.name.bright_yellow(),
                                     state.character.hit_points.to_string().bright_white(),
                                     state.character.max_hit_points.to_string().bright_white(),
                                     "AC".bright_yellow(),
                                     state.character.armor_class.to_string().bright_white());
                            println!("{}", "-".repeat(60).bright_blue());
                            
                            // Show player options
                            println!("\n{}", "What would you like to do?".bright_cyan());
                            let actions = vec![
                                "Take an action", 
                                "Roll a skill check", 
                                "Roll a dice", 
                                "Show character sheet",
                                "Save game",
                                "Return to main menu"
                            ];
                            
                            let action_choice = Select::with_theme(&ColorfulTheme::default())
                                .with_prompt("Choose an action")
                                .default(0)
                                .items(&actions)
                                .interact()?;
                            
                            match action_choice {
                                0 => {
                                    // Take an action
                                    let player_action: String = Input::with_theme(&ColorfulTheme::default())
                                        .with_prompt("What would you like to do? (describe your action)")
                                        .interact_text()?;
                                    
                                    let dm_response = process_player_action(&dungeon_master, &player_action, &mut state).await?;
                                    print_fancy_message("Dungeon Master:", "cyan");
                                    println!("{}", dm_response.bright_white());
                                },
                                1 => {
                                    // Roll a skill check
                                    let skills = vec![
                                        "Acrobatics", "Animal Handling", "Arcana", "Athletics", "Deception", 
                                        "History", "Insight", "Intimidation", "Investigation", "Medicine", 
                                        "Nature", "Perception", "Performance", "Persuasion", "Religion", 
                                        "Sleight of Hand", "Stealth", "Survival"
                                    ];
                                    
                                    let skill_index = Select::with_theme(&ColorfulTheme::default())
                                        .with_prompt("Choose a skill to check")
                                        .default(0)
                                        .items(&skills)
                                        .interact()?;
                                    
                                    let skill = skills[skill_index];
                                    let is_proficient = state.character.skills.get(skill).unwrap_or(&false);
                                    
                                    // Roll the d20
                                    let d20_results = roll_dice(1, 20);
                                    let roll_result = d20_results[0];
                                    
                                    // Print the roll
                                    print_fancy_message(&format!("{} Check", skill), "yellow");
                                    print_dice_roll("d20", &d20_results);
                                    
                                    // Get ability modifier
                                    let ability_mod = match skill {
                                        "Athletics" => (state.character.strength as i32 - 10) / 2,
                                        "Acrobatics" | "Sleight of Hand" | "Stealth" => (state.character.dexterity as i32 - 10) / 2,
                                        "Arcana" | "History" | "Investigation" | "Nature" | "Religion" => (state.character.intelligence as i32 - 10) / 2,
                                        "Animal Handling" | "Insight" | "Medicine" | "Perception" | "Survival" => (state.character.wisdom as i32 - 10) / 2,
                                        "Deception" | "Intimidation" | "Performance" | "Persuasion" => (state.character.charisma as i32 - 10) / 2,
                                        _ => 0,
                                    };
                                    
                                    // Calculate proficiency bonus
                                    let prof_bonus = match state.character.level {
                                        1..=4 => 2,
                                        5..=8 => 3,
                                        9..=12 => 4,
                                        13..=16 => 5,
                                        _ => 6,
                                    };
                                    
                                    // Calculate total
                                    let total = roll_result as i32 + ability_mod + if *is_proficient { prof_bonus } else { 0 };
                                    
                                    println!("Ability modifier: {}", ability_mod);
                                    if *is_proficient {
                                        println!("Proficiency bonus: +{}", prof_bonus);
                                    }
                                    println!("Total: {}", total.to_string().bright_green().bold());
                                    
                                    // Process the skill check with the DM
                                    let dm_response = roll_skill_check(&dungeon_master, skill, roll_result, &mut state).await?;
                                    print_fancy_message("Dungeon Master:", "cyan");
                                    println!("{}", dm_response.bright_white());
                                },
                                2 => {
                                    // Roll dice
                                    let dice_types = vec!["d4", "d6", "d8", "d10", "d12", "d20", "d100"];
                                    let dice_type_index = Select::with_theme(&ColorfulTheme::default())
                                        .with_prompt("Choose a dice type")
                                        .default(5) // d20 is default
                                        .items(&dice_types)
                                        .interact()?;
                                    
                                    let sides = match dice_types[dice_type_index] {
                                        "d4" => 4,
                                        "d6" => 6,
                                        "d8" => 8,
                                        "d10" => 10,
                                        "d12" => 12,
                                        "d20" => 20,
                                        "d100" => 100,
                                        _ => 6,
                                    };
                                    
                                    let num_dice: String = Input::with_theme(&ColorfulTheme::default())
                                        .with_prompt("How many dice?")
                                        .default("1".to_string())
                                        .interact_text()?;
                                    
                                    let num_dice = num_dice.parse::<u32>().unwrap_or(1);
                                    let results = roll_dice(num_dice, sides);
                                    
                                    print_fancy_message("Dice Roll", "yellow");
                                    print_dice_roll(&format!("{}d{}", num_dice, sides), &results);
                                },
                                3 => {
                                    // Show character sheet
                                    print_character_sheet(&state.character);
                                },
                                4 => {
                                    // Save game
                                    match save_game(&state) {
                                        Ok(_) => print_fancy_message("Game saved successfully!", "green"),
                                        Err(e) => print_fancy_message(&format!("Error saving game: {}", e), "red"),
                                    }
                                },
                                5 => {
                                    // Return to main menu
                                    print_fancy_message("Returning to main menu...", "blue");
                                    break;
                                },
                                _ => unreachable!(),
                            }
                        }
                    }
                    Err(_) => {
                        print_fancy_message("No saved adventure found or error loading save!", "red");
                        thread::sleep(Duration::from_secs(2));
                    }
                }
            },
            2 => {
                // View Rules & Commands
                print_fancy_message("D&D ADVENTURE RULES & COMMANDS", "blue");
                println!("{}", "Welcome to AI Dungeon Master!".bright_cyan());
                println!("{}", "Experience D&D 5th Edition in a text-based adventure with an AI Dungeon Master.".bright_white());
                
                println!("\n{}", "Game Features:".bright_yellow());
                println!("• Character creation with D&D 5e races, classes and abilities");
                println!("• Interactive storytelling with an AI Dungeon Master");
                println!("• Skill checks and dice rolling");
                println!("• Character progression");
                println!("• Save and load your adventure");
                
                println!("\n{}", "How to Play:".bright_yellow());
                println!("• Create a character or load a saved game");
                println!("• The DM will describe scenes and situations");
                println!("• Choose actions for your character to take");
                println!("• Roll skill checks when attempting difficult tasks");
                println!("• Engage in combat, exploration, and social interaction");
                
                println!("\n{}", "Commands during play:".bright_yellow());
                println!("• Take an action - Describe what your character does");
                println!("• Roll a skill check - Test your character's abilities");
                println!("• Roll dice - Roll any dice combination (1d20, 2d6, etc.)");
                println!("• Show character sheet - View your character's stats");
                println!("• Save game - Save your progress");
                
                println!("\n{}", "Basic D&D Concepts:".bright_yellow());
                println!("• Ability Scores - Six core attributes (STR, DEX, CON, INT, WIS, CHA)");
                println!("• Skill Checks - Roll d20 + ability modifier + proficiency (if applicable)");
                println!("• Difficulty Class (DC) - Target number to beat on skill checks");
                println!("• Hit Points (HP) - Your character's health");
                println!("• Armor Class (AC) - How difficult you are to hit in combat");
                
                println!("\n{}", "Press Enter to return to the main menu...".bright_cyan());
                let _: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("")
                    .allow_empty(true)
                    .interact_text()?;
            },
            3 => {
                // Quit
                print_fancy_message("Thanks for playing AI Dungeon Master!", "cyan");
                thread::sleep(Duration::from_secs(1));
                break;
            },
            _ => unreachable!(),
        }
    }
    
    Ok(())
}
