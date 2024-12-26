
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use tokio::time::sleep;
//use tokio::spawn_blocking;
use clap::{Parser, Subcommand};


#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
    id: u32,
    title: String,
    details: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    is_recurring: bool,
    frequency_minutes: Option<i64>,
}

#[derive(Default)]
struct AppState {
    tasks: Mutex<HashMap<u32, Task>>,
    next_id: Mutex<u32>,
}

#[derive(Parser)]
#[command(name = "Todo Task")]
#[command(about = "A CLI tool to manage tasks and reminder")]
struct CLI {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new Task
    Add {
        /// Title of the task
        title: String,
        /// Details of the task
        details: String,
        /// start time (ISO 8601 format, e.g., "2024-12-31T15:00:06")
        start_time: String,
        /// End time (ISO format)
        end_time: String,
        /// Whether the task is recurring
        #[arg(long)]
        recurring: bool,
        /// Frequency of recurrence in minutes (only for recurring tasks)
        frequency_minutes: Option<i64>,
    },
    /// List all tasks
    List,
}


// Implementation block for AppState struct
impl AppState {
    // intialize a add task to the state
    async fn add_task(&self, task: Task) -> u32 {
        let mut tasks = self.tasks.lock().await;
        let mut next_id = self.next_id.lock().await;


        // Assign task ID and increment next_id
        let task_id = *next_id;
        tasks.insert(task_id, Task { id :  task_id, ..task });
        *next_id +=1;

        task_id

    }

    async fn list_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.lock().await;
        tasks.values().cloned().collect()
    }


}

    // Send reminder at 5 mins before start and 2 mins before end
async fn schedule_reminders(task: Task, state: Arc<AppState>) {
    let reminder_time_start= task.start_time - chrono::Duration::minutes(5);
    let reminder_time_end= task.end_time - chrono::Duration::minutes(2);

    // wait until 5 mins before start time
    if let Ok(duration) = reminder_time_start.signed_duration_since(Utc::now()).to_std() {
        tokio::time::sleep(duration).await;
        println!("Reminder: '{}' starts in 5 minutes!", task.title);
    }

    //wait until 2 mins before end time
    if let Ok(duration) = reminder_time_end.signed_duration_since(Utc::now()).to_std() {
        tokio::time::sleep(duration).await;
        println!("Reminder: '{}' starts in 2 minutes!", task.title);
    }

    // clone the title field to reuse it after move
    let task_title = task.title.clone(); // clone the title


    // mark task as completed
    println!("Task '{}' is complete", task_title);

    // if the task is a recurring, schedule the next instance
    if task.is_recurring {
        if let Some(frequency) = task.frequency_minutes {
            let next_task = Task {
                start_time: task.start_time + chrono::Duration::minutes(frequency),
                end_time: task.end_time + chrono::Duration::minutes(frequency),
                // explicitly clone the field you need
                title: task.title.clone(),
                details: task.details.clone(),
                is_recurring: task.is_recurring,
                frequency_minutes: task.frequency_minutes,
                id: 0, 
            };

            // Schedule the next task after the frequency duration
            let delay_until_next_task = next_task.start_time - Utc::now();
            if let Ok(duration) = delay_until_next_task.to_std() {
                sleep(duration).await; // Wait until the next task's start time
            }

             // Add the next task to the state
             let task_id = state.add_task(next_task.clone()).await;

             println!("Next recurring task scheduled with ID: {}", task_id);
 
             // Spawn a task to schedule the next reminder
             tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async move {
                    schedule_reminders(next_task, Arc::clone(&state)).await;
                });
            });
            
            
        }
    }
}


#[tokio::main]
async fn main() {
    let cli = CLI::parse();
    let state = Arc::new(AppState::default());
    
    
    match cli.command{
        Commands::Add {
            title,
            details,
            start_time,
            end_time,
            recurring,
            frequency_minutes,
        } => {
            let start_time = start_time.parse::<DateTime<Utc>>()
            .expect("Invalid start time format. Use ISO 8601 format, e.g., '2024-12-31T15:00:06'");
            let end_time = end_time.parse::<DateTime<Utc>>()
            .expect("Invalid end time format. Use ISO 8601 format, e.g., '2024-12-31T15:00:06'");


            // Validation for start and end times
            if start_time <= Utc::now() {
                eprintln!("Error: Start time must be in the future.");
                return;
            }

            if end_time <= start_time {
                eprintln!("Error: End time must be after the start time.");
                return;
            }

            //Add task
            let task = Task {
                id: 0,
                title,
                details,
                start_time,
                end_time,
                is_recurring: recurring,
                frequency_minutes,
            };
            let _task_id = state.add_task(task.clone()).await;
            println!("Task '{}' added with ID: {}", task.title, task.id);

            tokio::spawn(async move {
                schedule_reminders(task, Arc::clone(&state)).await;
            });
        }
        Commands::List => {
            let tasks = state.list_tasks().await;
            for task in tasks {
                println!(
                    "ID: {}, Title: '{}', Details: '{}', Start: {}, End: {}, Recurring: {}",
                    task.id,
                    task.title,
                    task.details,
                    task.start_time,
                    task.end_time,
                    if task.is_recurring {"Yes"} else {"No"}
                );
            }
        }
    }
}