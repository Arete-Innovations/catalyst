#!/bin/bash

RED='\033[1;31m'
GREEN='\033[1;32m'
LIGHT_GREEN='\033[1;32m'
YELLOW='\033[1;33m'
BLUE='\033[1;34m'
MAGENTA='\033[1;35m'
CYAN='\033[1;36m'
WHITE='\033[1;37m'
NC='\033[0m'

MARKER=0

format_duration() {
  local seconds="$1"
  
  local ns=$(echo "$seconds * 1000000000" | bc | cut -d'.' -f1)
  local us=$(echo "$seconds * 1000000" | bc | cut -d'.' -f1) 
  local ms=$(echo "$seconds * 1000" | bc | cut -d'.' -f1)
  
  if (( ns < 1000 )); then
    echo "${ns}ns"
  elif (( us < 1000 )); then
    echo "${us}µs"
  elif (( ms < 1000 )); then
    echo "${ms}ms"
  else
    printf "%.2fs\n" "$seconds"
  fi
}


process_line() {
  local line="$1"
  if [[ $line =~ ([0-9]+\-[0-9]{4}-[0-9]{2}-[0-9]{2}\ [0-9]{2}:[0-9]{2}:[0-9]{2})\ \[([A-Z]+)\]\ (.*) ]]; then
    local time=${BASH_REMATCH[1]}
    local level=${BASH_REMATCH[2]}
    local message=${BASH_REMATCH[3]}
    
    if [[ "$level" == "TRACE" ]]; then
      local TIMED=$(echo $time | grep -Po "^[0-9]+\.[0-9]+")
      if [ $MARKER = 0 ]; then
        MARKER=$TIMED
      else
        local diff=$(echo "$TIMED - $MARKER" | bc)
        echo "Trace complete: $(format_duration "$diff")"
        MARKER=0
      fi
      local time_only=$(echo "$time" | grep -o '[0-9][0-9]:[0-9][0-9]:[0-9][0-9]')
      echo -e "$time_only [${MAGENTA}TRACE${NC}] $message"
      return
    fi
    
    local time_only=$(echo "$time" | grep -o '[0-9][0-9]:[0-9][0-9]:[0-9][0-9]')
    
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
        color=$WHITE
        ;;
      "DEBUG")
        color=$CYAN
        ;;
      "VOICE"|"SPEECH"|"VOIP")
        color=$BLUE
        ;;
    esac
    
    echo -e "$time_only [${color}$level${NC}] $message"
    
  else
    echo "$line"
  fi
}

LOG_FILE="$1"
NUM_LINES="200"

if [ ! -f "$LOG_FILE" ]; then
  echo "Log file $LOG_FILE does not exist. Creating it..."
  mkdir -p "$(dirname "$LOG_FILE")"
  touch "$LOG_FILE"
  echo "Created $LOG_FILE"
fi

echo "=== Showing history of $NUM_LINES lines of $LOG_FILE ==="
tail -f -n $NUM_LINES "$LOG_FILE" | while read line; do
  process_line "$line"
done
