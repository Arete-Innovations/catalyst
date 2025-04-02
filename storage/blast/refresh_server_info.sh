#!/bin/bash
# Auto-refreshing script for server info pane

LOG_FILE="storage/logs/server.log"
REFRESH_INTERVAL=5  # seconds

# Function to display server info in a cleaner format
display_server_info() {
  clear
  # Print header with timestamp
  echo -e "\033[1m\033[34mServer Information\033[0m (updated: $(date '+%H:%M:%S'))"
  echo "--------------------------------------------------------"
  
  if [ -f "$LOG_FILE" ]; then
    # Find the most recent configuration block
    if grep -q "Configured for" "$LOG_FILE"; then
      # Get the section between "Configured for" and "Fairings" or "engine"
      CONFIG_BLOCK=$(grep -A 20 "Configured for" "$LOG_FILE" | tail -n 21)
      
      # Display with simple formatting
      echo "$CONFIG_BLOCK" | while IFS= read -r line; do
        if [[ "$line" == *"Configured for"* ]]; then
          echo -e "\033[32m$line\033[0m"  # Green
        elif [[ "$line" == *">>"* ]]; then
          # Highlight the parameter name (between ">>" and ":")
          param=$(echo "$line" | sed -n 's/.*>> \([^:]*\):.*/\1/p')
          if [ -n "$param" ]; then
            # Replace the parameter with a colored version
            echo -e "\033[34m>>\033[0m \033[33m$param\033[0m:$(echo "$line" | sed -n 's/.*>> [^:]*:\(.*\)/\1/p')"
          else
            echo -e "\033[34m$line\033[0m"  # Blue for other >> lines
          fi
        elif [[ "$line" == *"Fairings"* ]]; then
          echo -e "\033[36m$line\033[0m"  # Cyan
        else
          echo "$line"  # Normal text
        fi
      done
    else
      echo -e "\033[33mNo server configuration found.\033[0m"
      echo "The server may not have started yet."
    fi
  else
    echo -e "\033[33mServer log file not found:\033[0m $LOG_FILE"
  fi
}

# Main refresh loop
while true; do
  display_server_info
  sleep $REFRESH_INTERVAL
done