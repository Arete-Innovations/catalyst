#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
LIGHT_GREEN='\033[1;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

MARKER=0



process_line() {
  local line="$1"
  if [[ $line =~ ([\]\[ 0-9.:-]+)\ \[([A-Z]{2,9})\]\ (.*) ]]; then
    local time=${BASH_REMATCH[1]}
    local level=${BASH_REMATCH[2]}
    local message=${BASH_REMATCH[3]}
    
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
      "TRACE")
        color=$MAGENTA
        local TIMED=$(echo $time | grep -Po "^[\d.]+")
        if [ $MARKER = 0 ]; then
          MARKER=$TIMED
        else
          echo "Trace complete: $(calc $TIMED-$MARKER)s"
          MARKER=0
        fi
        ;;
    esac
    
    echo -e "$time [${color}$level${NC}] $message"
    
  else
    echo "$line"
  fi
}

LOG_FILE="$1"
NUM_LINES="200"

if [ ! -f "$LOG_FILE" ]; then
  echo "Error: Log file $LOG_FILE does not exist."
  exit 1
fi

echo "=== Showing history of $NUM_LINES lines of $LOG_FILE ==="
tail -f -n $NUM_LINES "$LOG_FILE" | while read line; do
  process_line "$line"
done
