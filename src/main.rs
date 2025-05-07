use chrono::Local;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use dotenv::dotenv;
use indicatif::{ProgressBar, ProgressStyle};
use rig::{
    completion::{Chat, Message},
    providers::openai,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fs, path::Path, thread, time::Duration, io};

const SAVE_FILE: &str = "twenty_questions_save.json";
const CATEGORY_DESCRIPTIONS: [&str; 3] = [
    "Person: Someone real or fictional",
    "Place: A location or geographical feature",
    "Thing: An object, concept, or animal",
];

const TITLE_ART: &str = r#"
 __  __ _           _   ____                _           
|  \/  (_)_ __   __| | |  _ \ ___  __ _  __| | ___ _ __ 
| |\/| | | '_ \ / _` | | |_) / _ \/ _` |/ _` |/ _ \ '__|
| |  | | | | | | (_| | |  _ <  __/ (_| | (_| |  __/ |   
|_|  |_|_|_| |_|\__,_| |_| \_\___|\__,_|\__,_|\___|_|   
                                                        
"#;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct GameState {
    category: usize,
    secret_subject: String,
    questions_asked: usize,
    questions_remaining: usize,
    current_guess: Option<String>,
    history: Vec<Message>,
    has_won: bool,
    date_started: String,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            category: 1,
            secret_subject: String::new(),
            questions_asked: 0,
            questions_remaining: 20,
            current_guess: None,
            history: Vec::new(),
            has_won: false,
            date_started: Local::now().to_rfc3339(),
        }
    }
}

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
    println!("{}", "The 20 Questions Game".bright_purple().bold());
    println!("{}\n", "=".repeat(50).bright_blue());
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

fn get_category_prompt(category: usize) -> &'static str {
    match category {
        0 => "Think of a well-known person (real or fictional) that the player will try to guess.",
        1 => "Think of a specific place or geographical location that the player will try to guess.",
        2 => "Think of a specific object, concept, or animal that the player will try to guess.",
        _ => "Think of something that the player will try to guess through yes/no questions.",
    }
}

fn calculate_remaining_questions(questions_asked: usize) -> usize {
    if questions_asked >= 20 {
        0
    } else {
        20 - questions_asked
    }
}

/// Helper function to handle API calls with consistent error handling
async fn mind_reader_chat<C>(
    mind_reader: &C,
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
    
    match mind_reader.chat(prompt, history).await {
        Ok(response) => Ok(response),
        Err(e) => {
            print_fancy_message("The Mind Reader cannot respond...", "red");
            println!("Error: {}", e);
            Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                error_message,
            )))
        }
    }
}

async fn start_new_game(
    mind_reader: &impl Chat,
    category: usize,
) -> Result<GameState, Box<dyn Error>> {
    let mut state = GameState {
        category,
        date_started: Local::now().to_rfc3339(),
        questions_remaining: 20,
        ..Default::default()
    };
    
    // Create the category prompt
    let category_prompt = get_category_prompt(category);
    
    let subject_response = mind_reader_chat(
        mind_reader,
        category_prompt,
        vec![],
        "Failed to communicate with the Mind Reader",
        "The Mind Reader is thinking of something...",
        3000,
    )
    .await?;
    
    state.secret_subject = subject_response.clone();
    
    // Add to history
    state.history.push(Message::user(category_prompt));
    state.history.push(Message::assistant(&subject_response));
    
    // Add a follow-up message to make sure the AI remembers what it's thinking of
    let confirmation_prompt = "Remember, you're thinking of this subject. The player will now try to guess it through yes/no questions. Only respond with yes, no, or maybe to their questions.";
    
    let confirmation = mind_reader_chat(
        mind_reader,
        confirmation_prompt,
        state.history.clone(),
        "Failed to confirm with the Mind Reader",
        "The Mind Reader is getting ready...",
        1500,
    )
    .await?;
    
    state.history.push(Message::user(confirmation_prompt));
    state.history.push(Message::assistant(&confirmation));
    
    save_game(&state)?;
    
    Ok(state)
}

async fn ask_question(
    mind_reader: &impl Chat,
    question: &str,
    state: &mut GameState,
) -> Result<String, Box<dyn Error>> {
    state.questions_asked += 1;
    state.questions_remaining = calculate_remaining_questions(state.questions_asked);
    
    let question_prompt = format!(
        "The player asks: {}
Please answer with only 'Yes', 'No', or 'Maybe' (if truly ambiguous).",
        question
    );
    
    let answer = mind_reader_chat(
        mind_reader,
        &question_prompt,
        state.history.clone(),
        "Failed to get an answer from the Mind Reader",
        "The Mind Reader is considering the question...",
        1500,
    )
    .await?;
    
    state.history.push(Message::user(&question_prompt));
    state.history.push(Message::assistant(&answer));
    
    save_game(state)?;
    
    Ok(answer)
}

async fn make_final_guess(
    mind_reader: &impl Chat,
    guess: &str,
    state: &mut GameState,
) -> Result<bool, Box<dyn Error>> {
    state.current_guess = Some(guess.to_string());
    
    let guess_prompt = format!(
        "The player's final guess is: {}\nIs this correct? Please answer exactly \"yes\" or \"no\", nothing more.",
        guess
    );
    
    let judgement = mind_reader_chat(
        mind_reader,
        &guess_prompt,
        state.history.clone(),
        "Failed to get judgment from the Mind Reader",
        "The Mind Reader is judging your guess...",
        1500,
    )
    .await?;
    
    // Trim and convert to lowercase for more reliable comparison
    let judgement_clean = judgement.trim().to_lowercase();
    let correct = judgement_clean == "yes" || judgement_clean == "yes.";
    
    state.history.push(Message::user(&guess_prompt));
    state.history.push(Message::assistant(&judgement));
    
    if correct {
        state.has_won = true;
    }
    
    save_game(state)?;
    
    Ok(correct)
}

async fn reveal_answer(
    mind_reader: &impl Chat,
    state: &mut GameState,
) -> Result<String, Box<dyn Error>> {
    let answer_prompt = "Please reveal what you were thinking of and provide a brief description of it.";
    
    let answer = mind_reader_chat(
        mind_reader,
        answer_prompt,
        state.history.clone(),
        "Failed to get the answer from the Mind Reader",
        "The Mind Reader is revealing the answer...",
        2000,
    )
    .await?;
    
    state.history.push(Message::user(answer_prompt));
    state.history.push(Message::assistant(&answer));
    
    save_game(state)?;
    
    Ok(answer)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = dotenv().ok();
    let openai = openai::Client::from_env();
    
    let mind_reader = openai
        .agent("gpt-4o")
        .preamble("You are playing the role of the Mind Reader in a 20 Questions game. The player will try to guess what you're thinking of by asking up to 20 yes/no questions. When they ask a question, you must respond with only 'Yes', 'No', or 'Maybe' if it's truly ambiguous. You'll be given a category (person, place, or thing) and asked to think of something from that category. Remember what you've chosen throughout the game. Be fair and consistent with your answers. If the player makes a final guess, judge whether it matches what you were thinking.")
        .temperature(1.0)
        .build();
    
    // Main game loop
    loop {
        print_header();
        
        let selections = vec!["Start New Game", "Continue Saved Game", "View Instructions", "Quit"];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose an option:")
            .default(0)
            .items(&selections)
            .interact()?;
        
        match selection {
            0 => {
                // Start New Game
                print_fancy_message("Choose a category:", "cyan");
                let category = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Category")
                    .default(1)
                    .items(&CATEGORY_DESCRIPTIONS)
                    .interact()?;
                
                let mut state = start_new_game(&mind_reader, category).await?;
                
                print_fancy_message("The Mind Reader has chosen something!", "yellow");
                println!("{}", "I'm thinking of something from the chosen category.".bright_white());
                println!("{}", "Ask up to 20 yes/no questions to figure out what it is.".bright_white());
                
                // 20 Questions game loop
                loop {
                    println!("\n{}", "-".repeat(50).bright_blue());
                    println!("Questions asked: {} | Questions remaining: {}", 
                             state.questions_asked.to_string().yellow(),
                             state.questions_remaining.to_string().yellow());
                    println!("{}", "-".repeat(50).bright_blue());
                    
                    if state.questions_remaining == 0 {
                        print_fancy_message("You've used all 20 questions!", "yellow");
                        println!("{}", "Time to make your final guess.".bright_white());
                        
                        let final_guess: String = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("What is your final guess?")
                            .interact_text()?;
                        
                        let correct = make_final_guess(&mind_reader, &final_guess, &mut state).await?;
                        
                        if correct {
                            print_fancy_message("CORRECT! You guessed it!", "green");
                        } else {
                            print_fancy_message("INCORRECT! Better luck next time!", "red");
                        }
                        
                        let answer = reveal_answer(&mind_reader, &mut state).await?;
                        print_fancy_message("The Mind Reader reveals:", "cyan");
                        println!("{}", answer.bright_white());
                        
                        println!("\nWould you like to play again?");
                        let play_again = Select::with_theme(&ColorfulTheme::default())
                            .with_prompt("Choose an option")
                            .default(0)
                            .items(&["Yes", "No"])
                            .interact()?;
                        
                        if play_again == 0 {
                            break; // Break out of game loop to start a new game
                        } else {
                            return Ok(());
                        }
                    }
                    
                    // Get the player's question or guess
                    let input: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Your question (or type 'guess' to make your final guess)")
                        .interact_text()?;
                    
                    if input.trim().to_lowercase() == "guess" {
                        // Player wants to make a final guess
                        let final_guess: String = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("What is your final guess?")
                            .interact_text()?;
                        
                        let correct = make_final_guess(&mind_reader, &final_guess, &mut state).await?;
                        
                        if correct {
                            print_fancy_message("CORRECT! You guessed it!", "green");
                        } else {
                            print_fancy_message("INCORRECT! Better luck next time!", "red");
                        }
                        
                        let answer = reveal_answer(&mind_reader, &mut state).await?;
                        print_fancy_message("The Mind Reader reveals:", "cyan");
                        println!("{}", answer.bright_white());
                        
                        println!("\nWould you like to play again?");
                        let play_again = Select::with_theme(&ColorfulTheme::default())
                            .with_prompt("Choose an option")
                            .default(0)
                            .items(&["Yes", "No"])
                            .interact()?;
                        
                        if play_again == 0 {
                            break; // Break out of game loop to start a new game
                        } else {
                            return Ok(());
                        }
                    } else {
                        // Process the question
                        let answer = ask_question(&mind_reader, &input, &mut state).await?;
                        print_fancy_message("Mind Reader's answer:", "magenta");
                        println!("{}", answer.bright_white());
                    }
                }
            }
            1 => {
                // Continue Saved Game
                match load_game() {
                    Ok(mut state) => {
                        if state.secret_subject.is_empty() {
                            print_fancy_message("No saved game found!", "red");
                            thread::sleep(Duration::from_secs(2));
                            continue;
                        }
                        
                        print_fancy_message("Continuing your game...", "blue");
                        println!("Category: {} | Questions asked: {} | Questions remaining: {}", 
                                 CATEGORY_DESCRIPTIONS[state.category].yellow(),
                                 state.questions_asked.to_string().yellow(),
                                 state.questions_remaining.to_string().yellow());
                        
                        print_fancy_message("The Mind Reader is ready:", "yellow");
                        println!("{}", "I'm still thinking of the same thing. Continue asking yes/no questions!".bright_white());
                        
                        // Continue 20 Questions game loop
                        loop {
                            println!("\n{}", "-".repeat(50).bright_blue());
                            println!("Questions asked: {} | Questions remaining: {}", 
                                     state.questions_asked.to_string().yellow(),
                                     state.questions_remaining.to_string().yellow());
                            println!("{}", "-".repeat(50).bright_blue());
                            
                            if state.questions_remaining == 0 {
                                print_fancy_message("You've used all 20 questions!", "yellow");
                                println!("{}", "Time to make your final guess.".bright_white());
                                
                                let final_guess: String = Input::with_theme(&ColorfulTheme::default())
                                    .with_prompt("What is your final guess?")
                                    .interact_text()?;
                                
                                let correct = make_final_guess(&mind_reader, &final_guess, &mut state).await?;
                                
                                if correct {
                                    print_fancy_message("CORRECT! You guessed it!", "green");
                                } else {
                                    print_fancy_message("INCORRECT! Better luck next time!", "red");
                                }
                                
                                let answer = reveal_answer(&mind_reader, &mut state).await?;
                                print_fancy_message("The Mind Reader reveals:", "cyan");
                                println!("{}", answer.bright_white());
                                
                                println!("\nWould you like to play again?");
                                let play_again = Select::with_theme(&ColorfulTheme::default())
                                    .with_prompt("Choose an option")
                                    .default(0)
                                    .items(&["Yes", "No"])
                                    .interact()?;
                                
                                if play_again == 0 {
                                    break; // Break out of game loop to start a new game
                                } else {
                                    return Ok(());
                                }
                            }
                            
                            // Get the player's question or guess
                            let input: String = Input::with_theme(&ColorfulTheme::default())
                                .with_prompt("Your question (or type 'guess' to make your final guess)")
                                .interact_text()?;
                            
                            if input.trim().to_lowercase() == "guess" {
                                // Player wants to make a final guess
                                let final_guess: String = Input::with_theme(&ColorfulTheme::default())
                                    .with_prompt("What is your final guess?")
                                    .interact_text()?;
                                
                                let correct = make_final_guess(&mind_reader, &final_guess, &mut state).await?;
                                
                                if correct {
                                    print_fancy_message("CORRECT! You guessed it!", "green");
                                } else {
                                    print_fancy_message("INCORRECT! Better luck next time!", "red");
                                }
                                
                                let answer = reveal_answer(&mind_reader, &mut state).await?;
                                print_fancy_message("The Mind Reader reveals:", "cyan");
                                println!("{}", answer.bright_white());
                                
                                println!("\nWould you like to play again?");
                                let play_again = Select::with_theme(&ColorfulTheme::default())
                                    .with_prompt("Choose an option")
                                    .default(0)
                                    .items(&["Yes", "No"])
                                    .interact()?;
                                
                                if play_again == 0 {
                                    break; // Break out of game loop to start a new game
                                } else {
                                    return Ok(());
                                }
                            } else {
                                // Process the question
                                let answer = ask_question(&mind_reader, &input, &mut state).await?;
                                print_fancy_message("Mind Reader's answer:", "magenta");
                                println!("{}", answer.bright_white());
                            }
                        }
                    }
                    Err(_) => {
                        print_fancy_message("No saved game found or error loading save!", "red");
                        thread::sleep(Duration::from_secs(2));
                    }
                }
            }
            2 => {
                // View Instructions
                print_fancy_message("HOW TO PLAY", "blue");
                println!("{}", "Welcome to 20 Questions!".bright_cyan());
                println!("{}", "In this game, the Mind Reader thinks of something and you must guess what it is.".bright_white());
                println!("{}", "You can ask up to 20 yes/no questions to figure it out.".bright_white());
                println!("\n{}", "Game Features:".bright_yellow());
                println!("• Three categories: Person, Place, or Thing");
                println!("• Ask up to 20 questions that can be answered with yes or no");
                println!("• Make a final guess at any time");
                println!("• Automatic game saving");
                
                println!("\n{}", "Commands during play:".bright_yellow());
                println!("• Ask any yes/no question");
                println!("• Type 'guess' when you're ready to make your final guess");
                
                println!("\n{}", "Press Enter to return to the main menu...".bright_cyan());
                let _: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("")
                    .allow_empty(true)
                    .interact_text()?;
            }
            3 => {
                // Quit
                print_fancy_message("Thanks for playing 20 Questions!", "cyan");
                thread::sleep(Duration::from_secs(1));
                break;
            }
            _ => unreachable!(),
        }
    }
    
    Ok(())
}
