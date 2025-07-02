# sheet_watch

A Rust CLI tool that reads new rows from a Google Sheets tab named "Raw", normalizes each row, and appends the results to a local CSV file. Designed to run on Windows or Linux as a single executable that can be scheduled via cron or Task Scheduler.

## Quick Start

### Build Instructions

```bash
# Clone the repository
git clone <repository-url>
cd sheet_watch

# Build release version
cargo build --release

# The executable will be located at:
# Windows: target\release\sheet_watch.exe
# Linux: target/release/sheet_watch
```

### Requirements

- Rust 1.70+ (edition 2021)
- Google Cloud Platform service account with Sheets API access
- Google Sheet with a "Raw" tab containing data to process

## Configuration

1. Copy the example config file:
   ```bash
   cp config/config.example.toml config/config.toml
   ```

2. Edit `config/config.toml` with your specific values:
   - `sheet_id`: Your Google Sheets document ID
   - `raw_range`: The range to read from (e.g., "Raw!A2:Z")
   - Output CSV path and settings

## Authentication Setup

### Creating a Service Account

1. Go to the [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Enable the Google Sheets API
4. Create a service account:
   - Go to IAM & Admin > Service Accounts
   - Click "Create Service Account"
   - Provide a name and description
   - Skip role assignment for now
5. Create and download a JSON key:
   - Click on the created service account
   - Go to "Keys" tab
   - Click "Add Key" > "Create new key"
   - Choose JSON format and download

### Sharing the Sheet

1. Open your Google Sheet
2. Click the "Share" button
3. Add the service account email address (found in the JSON key file)
4. Give it "Viewer" permissions (read-only access)

### Configure Authentication

You have two options for authentication:

**Option 1: Place the service account key in the project root (Recommended)**
- Place your service account JSON key file in the project root directory
- The application will automatically detect and use any `.json` file
- Common names: `service-account-key.json`, `credentials.json`, `gcp-key.json`

**Option 2: Use environment variable**
```bash
# Windows PowerShell
$env:GOOGLE_APPLICATION_CREDENTIALS="C:\path\to\your\service-account-key.json"

# Windows Command Prompt
set GOOGLE_APPLICATION_CREDENTIALS=C:\path\to\your\service-account-key.json

# Linux/macOS
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/your/service-account-key.json"
```

**Note:** The service account JSON key is automatically added to `.gitignore` to prevent accidental commits.

## Usage

### Command Line Options

```bash
sheet_watch [OPTIONS]

Options:
    --sheet-id <SHEET_ID>      Google Sheets ID (overrides config)
    --raw-range <RANGE>        Raw range to read from (overrides config)
    --csv-path <PATH>          Path to output CSV file (overrides config)
    --once                     Run once then exit (don't run as scheduler)
    --log-level <LEVEL>        Log level: debug, info, warn, error [default: info]
    --config <PATH>            Path to config file [default: config/config.toml]
    -h, --help                 Print help
    -V, --version              Print version
```

### Examples

```bash
# Run once with default configuration
sheet_watch --once

# Run with custom sheet ID and log level
sheet_watch --sheet-id "1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms" --log-level debug --once

# Run with custom CSV output path
sheet_watch --csv-path "output/my_data.csv" --once
```

## Scheduling

### Windows Task Scheduler

Create a new task with the following XML configuration (save as `sheet_watch_task.xml`):

```xml
<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers>
    <CalendarTrigger>
      <Repetition>
        <Interval>PT15M</Interval>
        <StopAtDurationEnd>false</StopAtDurationEnd>
      </Repetition>
      <StartBoundary>2024-01-01T00:00:00</StartBoundary>
      <ExecutionTimeLimit>PT5M</ExecutionTimeLimit>
      <Enabled>true</Enabled>
      <ScheduleByDay>
        <DaysInterval>1</DaysInterval>
      </ScheduleByDay>
    </CalendarTrigger>
  </Triggers>
  <Actions>
    <Exec>
      <Command>C:\path\to\sheet_watch.exe</Command>
      <Arguments>--once</Arguments>
      <WorkingDirectory>C:\path\to\working\directory</WorkingDirectory>
    </Exec>
  </Actions>
</Task>
```

Import the task:
```cmd
schtasks /create /xml "sheet_watch_task.xml" /tn "SheetWatch"
```

### Linux Cron

Add to your crontab (`crontab -e`):

```bash
# Run every 15 minutes
*/15 * * * * /path/to/sheet_watch --once >> /var/log/sheet_watch.log 2>&1

# Run every hour at minute 0
0 * * * * /path/to/sheet_watch --once

# Run daily at 6 AM
0 6 * * * /path/to/sheet_watch --once
```

## How It Works

1. **Load State**: Reads `state.json` to determine the last processed row (starts at 0)
2. **Fetch New Rows**: Queries Google Sheets for new rows starting after the last processed row
3. **Normalize Data**: Transforms each raw row into a standardized format
4. **Append to CSV**: Writes normalized rows to the configured CSV file
5. **Update State**: Saves the new last processed row number for the next run
6. **Safe Re-runs**: Multiple executions won't duplicate data or modify the source sheet

## Project Structure

```
src/
├── main.rs          # Entry point, argument parsing, logging
├── args.rs          # CLI argument definitions
├── auth.rs          # Google Sheets authentication
├── cfg.rs           # Configuration management
├── csv_sink.rs      # CSV file writing
├── job.rs           # Main job execution logic
├── sheets.rs        # Google Sheets API integration
├── state.rs         # State persistence
└── transform.rs     # Data normalization

config/
└── config.example.toml  # Configuration template
```

## License

[Add your license information here] 