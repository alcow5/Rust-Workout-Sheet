# sheet_watch

An intelligent Rust CLI tool that automatically discovers and processes workout data from Google Sheets. Designed For my specific coachs Google sheet and the format we use together for my training regiment, it dynamically finds all workout "Block" tabs, intelligently detects the optimal data range for each block, and extracts structured workout data to CSV format.

## 🧠 **Intelligent Auto-Discovery Features**

- **🔍 Automatic Block Discovery**: Finds all "Block 1", "Block 2", ..., "Block N" tabs automatically
- **📏 Dynamic Range Detection**: Each block gets optimal column range based on actual week data
- **⚡ Efficient Processing**: Only fetches columns that contain actual workout data
- **🔮 Future-Proof**: New blocks are automatically discovered and processed
- **📊 Multi-Week Support**: Handles 4-week, 6-week, 8-week, or any size training blocks
- **💪 Workout-Aware**: Understands prescribed vs actual workout data structure

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
- Google Sheet with "Block N" tabs containing workout data

## Configuration

1. Copy the example config file:
   ```bash
   cp config/config.example.toml config/config.toml
   ```

2. Edit `config/config.toml` with your specific values:
   ```toml
   sheet_id = "YOUR_GOOGLE_SHEETS_ID"
   # Auto-discover all Block tabs in the spreadsheet with optimal column ranges
   # Each block's column extent is dynamically detected based on actual week data
   # Fallback template used only if dynamic detection fails
   block_range_template = "Block {}!A1:BZ"
   # Optional: specify particular blocks to process instead of auto-discovering all
   # specific_blocks = [1, 2, 5]  # Uncomment to process only specific blocks
   
   state_path = "state.json"
   
   [output_csv]
   path = "normalized/normalized.csv"
   ensure = true
   ```

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
    --raw-range <RANGE>        Legacy: single range to read (overrides auto-discovery)
    --csv-path <PATH>          Path to output CSV file (overrides config)
    --once                     Run once then exit (don't run as scheduler)
    --log-level <LEVEL>        Log level: debug, info, warn, error [default: info]
    --config <PATH>            Path to config file [default: config/config.toml]
    -h, --help                 Print help
    -V, --version              Print version
```

### Examples

```bash
# Run once with auto-discovery (discovers all Block tabs automatically)
sheet_watch --once

# Run with custom sheet ID and debug logging
sheet_watch --sheet-id "1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms" --log-level debug --once

# Run with custom CSV output path
sheet_watch --csv-path "output/my_workouts.csv" --once
```

## How It Works

### 🧠 **Intelligent Discovery Process**

1. **Auto-Discover Blocks**: Scans the Google Sheet for all tabs matching "Block N" pattern
2. **Analyze Each Block**: For each block, samples the first few rows to detect:
   - Week structure (date headers like "5/19/2025", "5/26/2025")
   - Column extent (where the actual data ends)
   - Day structure (DAY 1, DAY 2, etc.)
3. **Optimize Ranges**: Calculates the minimal column range needed (e.g., `A1:BX` for 6-week blocks, `A1:CJ` for 9-week blocks)
4. **Smart Extraction**: Fetches only the necessary data, avoiding empty columns

### 📊 **Workout Data Processing**

1. **Load State**: Reads `state.json` to track progress per block independently
2. **Fetch New Rows**: Queries Google Sheets for new rows in each block's optimal range
3. **Parse Structure**: Understands the multi-week horizontal layout:
   - **Prescribed Data**: "find 5 RPE", "base on max", rep ranges
   - **Actual Data**: Real weights, sets, reps, RPE values, notes
4. **Generate Records**: Creates individual workout records with:
   - Block information
   - Week dates and progression
   - Exercise details (prescribed vs actual)
   - Calculated workout dates (Monday + day offsets)
5. **Append to CSV**: Writes normalized data to structured CSV format
6. **Update State**: Saves progress per block for incremental processing

### 🔄 **Incremental & Safe**

- **Per-Block State**: Each block tracks its own progress independently
- **Safe Re-runs**: Multiple executions won't duplicate data
- **Non-Destructive**: Never modifies the source Google Sheet
- **Future-Ready**: New blocks (Block 26, 27, etc.) are automatically discovered

## Scheduling

### Windows Task Scheduler

Create a new task with the following XML configuration (save as `sheet_watch_task.xml`):

```xml
<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers>
    <CalendarTrigger>
      <Repetition>
        <Interval>PT1H</Interval>
        <StopAtDurationEnd>false</StopAtDurationEnd>
      </Repetition>
      <StartBoundary>2024-01-01T00:00:00</StartBoundary>
      <ExecutionTimeLimit>PT15M</ExecutionTimeLimit>
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
# Run every hour
0 * * * * /path/to/sheet_watch --once >> /var/log/sheet_watch.log 2>&1

# Run daily at 6 AM
0 6 * * * /path/to/sheet_watch --once

# Run twice daily (morning and evening)
0 6,18 * * * /path/to/sheet_watch --once
```

## Sample Output Data

The CSV output contains rich workout data with both prescribed and actual values:

```csv
id,block_name,week_start_date,week_number,day_number,workout_date,exercise_name,record_type,sets,reps,load,rpe,notes
block24_w1_d1_lowbar_squats_prescribed,Block 24,5/19/2025,1,1,5/19/2025,Lowbar Squats w/belt,prescribed,3,7,find,5,
block24_w1_d1_lowbar_squats_actual,Block 24,5/19/2025,1,1,5/19/2025,Lowbar Squats w/belt,actual,3,7,350.0,5,
block24_w2_d1_lowbar_squats_prescribed,Block 24,5/26/2025,2,1,5/26/2025,Lowbar Squats w/belt,prescribed,3,7,find,6,
block24_w2_d1_lowbar_squats_actual,Block 24,5/26/2025,2,1,5/26/2025,Lowbar Squats w/belt,actual,3,7,375.0,7,
```

## Project Structure

```
src/
├── main.rs          # Entry point, argument parsing, logging setup
├── args.rs          # CLI argument definitions and parsing
├── auth.rs          # Google Sheets OAuth2 authentication
├── cfg.rs           # Configuration management and validation
├── csv_sink.rs      # CSV file writing and management
├── job.rs           # Main orchestration and block processing
├── sheets.rs        # Google Sheets API integration + auto-discovery
├── state.rs         # State persistence and per-block tracking
└── transform.rs     # Workout data parsing and normalization

config/
└── config.example.toml  # Configuration template

normalized/
└── normalized.csv       # Output workout data (auto-created)
```

## Advanced Configuration

### Processing Specific Blocks

To process only certain blocks instead of auto-discovering all:

```toml
# Process only blocks 1, 5, and 10
specific_blocks = [1, 5, 10]
```

### Legacy Single Range Mode

For backwards compatibility with non-block sheets:

```toml
# This disables auto-discovery and uses a single range
raw_range = "Data!A2:Z"
```

## Troubleshooting

### Common Issues

**"No block tabs found"**
- Ensure your sheet has tabs named "Block 1", "Block 2", etc.
- Check that the service account has access to the sheet

**"Failed to detect extent"**
- Verify the block has proper week structure with date headers
- Check that data starts from row 1 (headers in first few rows)

**"Authentication failed"**
- Verify service account JSON key is in the project root
- Ensure the service account email has Viewer access to the sheet
- Check that Google Sheets API is enabled in your GCP project

### Debug Mode

Run with debug logging to see detailed discovery and processing information:

```bash
sheet_watch --once --log-level debug
```

This will show:
- Which blocks are discovered
- Optimal ranges detected for each block
- Week structure analysis
- Data extraction details

## Performance

- **Efficient**: Only fetches necessary columns for each block
- **Scalable**: Handles 25+ blocks with thousands of workout records
- **Optimized**: Minimal API calls through intelligent range detection
- **Fast**: Typical run processes all blocks in 30-60 seconds

## License

[Add your license information here] 
