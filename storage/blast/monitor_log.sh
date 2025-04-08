#!/bin/bash
clear
tput civis

extract_section() {
    local logfile="$1"
    local section="$2"
    local strip_colors="$3"
    local start_pattern=""
    local end_pattern=""
    
    case "$section" in
        routes)
            start_pattern="📬.*Routes"
            end_pattern="🥅.*Catchers"
            ;;
        catchers)
            start_pattern="🥅.*Catchers"
            end_pattern="📡.*Fairings"
            ;;
        fairings)
            start_pattern="📡.*Fairings"
            end_pattern="🛡️.*Shield"
            ;;
        shield)
            start_pattern="🛡️.*Shield"
            end_pattern="📐.*Templating"
            ;;
        templating)
            start_pattern="📐.*Templating"
            end_pattern="✨.*Sparks"
            ;;
        sparks)
            start_pattern="✨.*Sparks"
            end_pattern="🚀"
            ;;
        all)
            # Special handling for 'all' - we'll ignore headers later
            start_pattern="📬.*Routes"
            end_pattern="🚀"
            ;;
        *)
            echo "Error: Unknown section '$section'"
            echo "Valid sections: routes, catchers, fairings, shield, templating, sparks, all"
            exit 1
            ;;
    esac
    
    local temp_file=$(mktemp)
    
    grep -n "$start_pattern" "$logfile" | tail -1 > "$temp_file.starts"
    
    if [ -s "$temp_file.starts" ]; then
        start_line=$(cut -d: -f1 "$temp_file.starts")
        
        start_line=$((start_line + 1))
        
        end_line=$(tail -n +"$start_line" "$logfile" | grep -n "$end_pattern" | head -1 | cut -d: -f1)
        
        if [ -n "$end_line" ]; then
            end_line=$((start_line + end_line - 1))
            
            sed -n "${start_line},${end_line}p" "$logfile" | sed '$d' > "$temp_file.raw"
        else
            tail -n +"$start_line" "$logfile" > "$temp_file.raw"
        fi
        
        grep -v -E '(📬|🥅|📡|🛡️|📐|✨|🚀)' "$temp_file.raw" | sed 's/^[[:space:]]*//' > "$temp_file"
        rm -f "$temp_file.raw"
    else
        echo "Error: Section '$section' not found in log file" > "$temp_file"
    fi
    
    rm -f "$temp_file.starts"
    
    if [ "$strip_colors" = "true" ]; then
        sed 's/\x1B\[[0-9;]*[a-zA-Z]//g' "$temp_file"
    else
        cat "$temp_file"
    fi
    
    rm -f "$temp_file"
}

section="${1:-routes}"
logfile="${2:-storage/logs/server.log}"
interval="${3:-1}"
strip_colors="${4:-false}"

if [ ! -f "$logfile" ]; then
    echo "Error: File '$logfile' not found"
    exit 1
fi

output_file=$(mktemp)
prev_output_file=$(mktemp)

trap 'rm -f "$output_file" "$prev_output_file"; echo -e "\nMonitoring stopped."; exit 0' EXIT INT TERM

extract_section "$logfile" "$section" "$strip_colors" > "$output_file"
cat "$output_file"
cp "$output_file" "$prev_output_file"

last_modified=$(stat -c %Y "$logfile")
last_size=$(stat -c %s "$logfile")

while true; do
    sleep "$interval"
    
    current_modified=$(stat -c %Y "$logfile")
    current_size=$(stat -c %s "$logfile")
    
    if [ "$current_modified" != "$last_modified" ] || [ "$current_size" != "$last_size" ]; then
        last_modified=$current_modified
        last_size=$current_size
        extract_section "$logfile" "$section" "$strip_colors" > "$output_file"
        if ! cmp -s "$output_file" "$prev_output_file"; then
            clear
            cat "$output_file"
            cp "$output_file" "$prev_output_file"
        fi
    fi
done
