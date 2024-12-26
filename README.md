# 1. Intoduction

# Todo CLI Tool

The **Todo CLI Tool** is a cross-platform, statically linked command-line application designed to help professionals manage tasks and reminders efficiently. With Google Calendar integration and recurring task support, the tool is perfect for staying organized on Linux, Windows, and Mac.


## 2. Usage Instructions

### Add a New Task
Add a task with specific start and end times, and optionally make it recurring:
```bash
todo_task add --title "Team Meeting" --details "Discuss project updates" --start_time "2024-12-31T15:00:00Z" --end_time "2024-12-31T16:00:00Z" --recurring --frequency_minutes 1440
```
View all Tasks and their details
```bash
todo_task list
```
View all available commands and flags:
```bash
todo_task --help
```


---

#### **3. Target Group Benefits**
```markdown
```
## Target Group Benefits

This tool is ideal for:
- **Busy Professionals**: Simplifies task management with automated reminders and recurring tasks.
- **Students**: Stay on top of assignments, study schedules, and personal goals.
- **Healthcare Workers**: Manage recurring tasks like medication schedules.

### Key Features:
1. **Automated Reminders**:
   - Get notified 5 minutes before a task starts.
   - Receive a final reminder 2 minutes before the task ends.

2. **Recurring Task Support**:
   - Easily manage repeated activities, such as daily stand-ups or prescription schedules.

3. **Google Calendar Integration**:
   - Synchronize tasks with Google Calendar for seamless access across devices.

4. **Cross-Platform**:
   - Works on Linux, Windows, and MacOS.
