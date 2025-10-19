#!/bin/bash

#
# A script to benchmark a file upload service with concurrent uploads.
# For each specified file size, it uploads files in parallel until a
# total data goal is met, then calculates the throughput.
#

# --- Configuration ---
# The base URL of your file upload service.
ENDPOINT="http://localhost:3000"

# An array of file sizes to test. You can use K, M, G suffixes.
SIZES=("1M" "10M" "50M" "1G")

# The number of concurrent uploads to perform.
CONCURRENCY=10

# The total amount of data to upload per test, in Gigabytes.
TOTAL_UPLOAD_GOAL_GB=20

# The name of the temporary file to be created for uploads.
TEMP_FILE="benchmark_temp_file.dat"

# --- Helper Functions ---

# Converts a size string (e.g., "50M") to bytes.
size_to_bytes() {
    local size_str=$1
    local num=$(echo "$size_str" | sed 's/[^0-9]*//g')
    local unit=$(echo "$size_str" | sed 's/[0-9]*//g' | tr '[:lower:]' '[:upper:]')
    local multiplier=1

    case "$unit" in
        K) multiplier=1024 ;;
        M) multiplier=$((1024*1024)) ;;
        G) multiplier=$((1024*1024*1024)) ;;
    esac

    echo $(($num * $multiplier))
}

# --- Main Script ---

# Prerequisite check: bc is needed for floating point math
if ! command -v bc &> /dev/null; then
    echo "ERROR: The 'bc' command is not found. Please install it to run this script."
    exit 1
fi

echo "Starting CONCURRENT file upload benchmark against: $ENDPOINT"
echo "Concurrency Level: $CONCURRENCY"
echo "Total Upload Goal per Test: ${TOTAL_UPLOAD_GOAL_GB}GB"
echo "----------------------------------------------------"

# Convert goal from GB to bytes for calculations
TOTAL_GOAL_BYTES=$(echo "$TOTAL_UPLOAD_GOAL_GB * 1024 * 1024 * 1024" | bc)

# Ensure no old temp file exists before starting.
rm -f "$TEMP_FILE"

# Loop through each defined size.
for size in "${SIZES[@]}"; do
    echo "-> Testing with file size: ${size}"

    file_size_bytes=$(size_to_bytes "$size")
    
    # Calculate how many uploads are needed to meet the goal
    num_uploads=$(echo "scale=0; $TOTAL_GOAL_BYTES / $file_size_bytes" | bc)
    if [ "$num_uploads" -lt 1 ]; then
        echo "   Skipping: File size ${size} is larger than the total goal of ${TOTAL_UPLOAD_GOAL_GB}GB."
        echo "----------------------------------------------------"
        continue
    fi
    
    echo "   Calculated uploads needed to reach goal: $num_uploads"

    # Create a temporary file of the specified size with random data.
    echo "   Generating $TEMP_FILE (${size})..."
    dd if=/dev/urandom of="$TEMP_FILE" bs="$size" count=1 &> /dev/null
    if [ $? -ne 0 ]; then
        echo "   ERROR: Failed to create test file of size ${size}. Skipping."
        echo "----------------------------------------------------"
        continue
    fi

    echo "   Starting $num_uploads concurrent uploads..."

    # Start timer (with nanosecond precision)
    start_time=$(date +%s.%N)

    # Use a loop to spawn background processes for uploads
    pids=()
    for i in $(seq 1 "$num_uploads"); do
        # Make the target URL unique for each upload to avoid server-side conflicts
        TARGET_URL="${ENDPOINT}/upload_${size}_${i}.dat"

        # Run curl in the background. Discard all output to keep the console clean.
        curl -s -o /dev/null -T "$TEMP_FILE" "$TARGET_URL" &
        pids+=($!)

        # If we've hit the concurrency limit, wait for a job to finish before starting a new one
        if [ ${#pids[@]} -ge "$CONCURRENCY" ]; then
            wait -n  # Wait for any background job to exit
            # Remove finished PIDs from the array (optional, but good practice)
            for j in "${!pids[@]}"; do
                if ! kill -0 "${pids[j]}" 2>/dev/null; then
                    unset 'pids[j]'
                fi
            done
        fi
    done

    # Wait for all remaining background jobs to complete
    wait "${pids[@]}"

    # Stop timer
    end_time=$(date +%s.%N)

    # Calculate results
    duration=$(echo "$end_time - $start_time" | bc)
    total_data_uploaded_bytes=$(echo "$num_uploads * $file_size_bytes" | bc)
    total_data_uploaded_mb=$(echo "scale=2; $total_data_uploaded_bytes / 1024 / 1024" | bc)
    throughput_mbps=$(echo "scale=2; $total_data_uploaded_mb / $duration" | bc)

    echo "   --- Results ---"
    echo "   Total data uploaded: ${total_data_uploaded_mb} MB"
    echo "   Total time: ${duration} seconds"
    echo "   Throughput: ${throughput_mbps} MB/s"

    # Clean up the temporary file.
    rm "$TEMP_FILE"

    echo "----------------------------------------------------"
done

echo "Benchmark finished."
