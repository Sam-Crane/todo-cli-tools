use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio::runtime::Runtime;
use google_calendar3::{api::Event, CalendarHub};
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};
use clap::{Parser, Subcommand};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::{Client, connect::HttpConnector};
use hyper::body::Body;

type HyperClient = Client<hyper_rustls::HttpsConnector<HttpConnector>, Body>;

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
#[command(about = "A CLI tool to manage tasks and reminder, integrated with Google Calendar")]
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
    // Remove a task by its ID
    Remove {
        /// ID of the task to be removed
        id: u32,
    },
    /// Sync tasks with Google Calendar
    Sync,
}


// Implementation block for AppState struct
impl AppState {
    // intialize a add task to the state
    pub async fn add_task(&self, task: Task) -> u32 {
        let mut tasks = self.tasks.lock().await;
        let mut next_id = self.next_id.lock().await;
        // Assign task ID and increment next_id
        let task_id = *next_id;
        *next_id +=1;
        tasks.insert(task_id, task);
        task_id
    }

    pub async fn list_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.lock().await;
        tasks.values().cloned().collect()
    }

    // Adding the remove task method
    pub async fn remove_task(&self, task_id: u32) -> Option<Task> {
        let mut tasks = self.tasks.lock().await;
        tasks.remove(&task_id)
    }
}

// Send reminder at 5 mins before start and 2 mins before end
async fn schedule_reminders(task: Task, state: Arc<AppState>) {
    let reminder_time_start = task.start_time - chrono::Duration::minutes(5);
    let reminder_time_end = task.end_time - chrono::Duration::minutes(2);
    let now = Utc::now();

    // wait until 5 mins before start time
    if reminder_time_start > now {
        if let Ok(duration) = reminder_time_start.signed_duration_since(now).to_std() {
            //tokio::time::
            sleep(duration).await;
            println!("Reminder: '{}' starts in 5 minutes!", task.title);
        }
    }

    //wait until 2 mins before end time
    if reminder_time_end > now {
        if let Ok(duration) = reminder_time_end.signed_duration_since(now).to_std() {
            //tokio::time::
            sleep(duration).await;
            println!("Reminder: '{}' ends in 2 minutes!", task.title);
        }
    }

    // clone the title field to reuse it after move
    let task_title = task.title.clone(); // clone the title
    // mark task as completed
    println!("Task '{}' is complete", task_title);

    // if the task is a recurring, schedule the next instance
    if task.is_recurring {
        if let Some(frequency) = task.frequency_minutes {
            let next_task = Task {
                id: 0,
                title: task.title.clone(),
                details: task.details.clone(),
                start_time: task.start_time + chrono::Duration::minutes(frequency),
                end_time: task.end_time + chrono::Duration::minutes(frequency),
                is_recurring: true,
                frequency_minutes: Some(frequency),
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
            //tokio::spawn(schedule_reminders(next_task, state.clone()));
            tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async move {
                    schedule_reminders(next_task, Arc::clone(&state)).await;
                });
            }); 
        }
    }
}
async fn authenticate() -> Result<CalendarHub<HyperClient>, Box<dyn std::error::Error>> {
    // Define the connector for hyper
    let rt = Runtime::new().expect("Failed to create Tokio runtime");

    let https_connector = HttpsConnectorBuilder::new()
        .with_native_roots()?
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .build();
    
    let hyper_client = HyperClient::builder(rt.handle().clone()).build();
        
    // Set up the authenticator
    let secret = yup_oauth2::read_application_secret("credentials.json")
        .await
        .expect("Failed to read credentials.json");
    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk("token_store.json")
        .build()
        .await?;

    //Create the CalendarHub
    Ok(CalendarHub::new(hyper_client, auth))
}

async fn add_to_google_calendar(task: &Task) -> Result<(), Box<dyn std::error::Error>> {
    // Authenticate with Google Calendar
    let hub = authenticate().await?;

    // Create a Google Calendar event
    let event = Event {
        summary: Some(task.title.clone()),
        description: Some(task.details.clone()),
        start: Some(google_calendar3::api::EventDateTime {
            date_time: Some(task.start_time),
            time_zone: Some("UTC".to_string()),
            ..Default::default()
        }),
        end: Some(google_calendar3::api::EventDateTime {
            date_time: Some(task.end_time),
            time_zone: Some("UTC".to_string()),
            ..Default::default()
        }),
        recurrence: task.is_recurring.then(|| {
            vec![format!("RRULE:FREQ=MINUTELY;INTERVAL={}", task.frequency_minutes.unwrap())]
        }),
        ..Default::default()
    };

    // Attempt to insert the event into Google Calendar
    let result = hub.events().insert(event, "primary").doit().await?;
    match result {
        Ok(_) => Ok(println!("Task successfully added to Google Calendar.")),
        Err(e) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to add task to Google Calendar: {:?}", e),
        ))),
    }
    
}

async fn sync_from_google_calendar(hub: &CalendarHub<HyperClient>, state: &AppState) -> Result<(), Box<dyn std::error::Error>> {
    let result = hub.events().list("primary").max_results(10).doit().await?;
    if let Some(items) = result.1.items {
        for event in items {
            if let (Some(summary), Some(start), Some(end)) = (
                event.summary.as_ref(),
                event.start.as_ref().and_then(|s| s.date_time.as_ref()),
                event.end.as_ref().and_then(|e| e.date_time.as_ref()),
            ) {
                let start_time = DateTime::parse_from_rfc3339(start)?.with_timezone(&Utc);
                let end_time = DateTime::parse_from_rfc3339(end)?.with_timezone(&Utc);
                let task = Task {
                    id: 0,
                    title: summary,
                    details: event.description.unwrap_or_default(),
                    start_time,
                    end_time,
                    is_recurring: false,
                    frequency_minutes: None,
                };
                    state.add_task(task).await;
            }
        }
    }
    println!("Tasks synchronized from Google Calendar.");
    Ok(())
}

//Main Application ENtry
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

            // Add the task to the state and get the task_id
            let _task_id = state.add_task(task.clone()).await;
            println!("Task '{}' added with ID: {}", task.title, task.id);

            if let Err(e) = add_to_google_calendar(&task).await {
                eprintln!("Error adding task to Google Calendar: {:?}", e);
            }

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

        Commands::Remove { id } => {
            if let Some(removed_task) = state.remove_task(id).await {
                println!("Removed task: {:?}", removed_task);
            } else {
                println!("Task with ID {} not found.", id);
            }
        }

        Commands::Sync => {
            // Synchronize tasks with Google Calendar
            let hub = authenticate().await.unwrap();
            if let Err(e) = sync_from_google_calendar(&hub, &state).await {
                eprintln!("Failed to sync tasks from Google Calendar: {:?}", e);
            }
            
        }
    }
}