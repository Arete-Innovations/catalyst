#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
LIGHT_GREEN='\033[1;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

process_line() {
  local line="$1"
  if [[ $line =~ \[([0-9]{4}-[0-9]{2}-[0-9]{2})[[:space:]]([0-9]{2}:[0-9]{2}:[0-9]{2})\][[:space:]]\[([A-Z]+)\] ]]; then
    local time=${BASH_REMATCH[2]}
    local level=${BASH_REMATCH[3]}
    
    local message=$(echo "$line" | sed -E 's/\[[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}\] \[[A-Z]+\] //')
    
    local color=$NC
    case $level in
      "SUCCESS")
        color=$GREEN
        ;;
      "ERROR")
        color=$RED
        ;;
      "WARNING"|"WARN")
        color=$YELLOW
        ;;
      "INFO")
        color=$LIGHT_GREEN
        ;;
      "DEBUG")
        color=$CYAN
        ;;
    esac
    
    echo -e "$time ${color}$message${NC}"
  else
    echo "$line"
  fi
}

LOG_FILE="storage/blast/blast.log"
NUM_LINES="200"

if [ ! -f "$LOG_FILE" ]; then
  echo "Error: Log file $LOG_FILE does not exist."
  exit 1
fi

echo "=== Showing history of $NUM_LINES lines of $LOG_FILE ==="
tail -f -n $NUM_LINES "$LOG_FILE" | while read line; do
  process_line "$line"
done
