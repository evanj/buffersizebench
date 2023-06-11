package main

import (
	"bufio"
	"encoding/csv"
	"fmt"
	"os"
	"regexp"
	"strconv"
	"time"
)

// var useBufferWriteBufferLine = regexp.MustCompile(`^## use_buffer=([^;]+); write_buffer_bytes=(\d+)`)
var runTypeLine = regexp.MustCompile(`^use_buffer=([^;]+); write_buffer_bytes=(\d+); type=([a-zA-Z]+)`)
var fileLine = regexp.MustCompile(`^use_buffer=([^;]+); (/dev/[^:]+):`)
var resultLine = regexp.MustCompile(`^buf_size=(\d+); duration=(\d+\.\d+s); num_syscalls=(\d+); (\d+\.\d+) MiB/s; \d+\.\d+ syscalls/s; short_reads=(\d+)`)

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintf(os.Stderr, "Usage: parsetocsv [machine_configuration_value]\n")
		fmt.Fprintf(os.Stderr, "    parsetocsv reads from stdin and writes to stdout\n")
	}
	machineConfiguration := os.Args[1]
	useBuffer := false
	writeBufferBytes := 0
	connectionType := ""

	results := csv.NewWriter(os.Stdout)
	results.Write([]string{"machine_configuration", "use_buffer", "write_buffer_bytes", "connection_type", "read_buffer_bytes", "duration_sec", "num_syscalls", "short_reads", "throughput (MiB/s)"})

	scanner := bufio.NewScanner(os.Stdin)
	for scanner.Scan() {
		// matches := useBufferWriteBufferLine.FindStringSubmatch(scanner.Text())
		// if len(matches) > 0 {
		// 	var err error
		// 	useBuffer, err = strconv.ParseBool(matches[1])
		// 	if err != nil {
		// 		panic(err)
		// 	}

		// 	writeBufferBytes, err = strconv.Atoi(matches[2])
		// 	if err != nil {
		// 		panic(err)
		// 	}
		// 	// fmt.Printf("use_buffer=%t; write_buffer_bytes=%d; line=%s\n", useBuffer, writeBufferBytes, scanner.Text())
		// 	continue
		// }

		matches := fileLine.FindStringSubmatch(scanner.Text())
		if len(matches) > 0 {
			var err error
			useBuffer, err = strconv.ParseBool(matches[1])
			if err != nil {
				panic(err)
			}

			writeBufferBytes = 0
			connectionType = "file_" + matches[2]
			// fmt.Printf("use_buffer=%t; write_buffer_bytes=%d; connection_type=%s; line=%s\n", useBuffer, writeBufferBytes, connectionType, scanner.Text())
			continue
		}

		matches = runTypeLine.FindStringSubmatch(scanner.Text())
		if len(matches) > 0 {
			var err error
			useBuffer, err = strconv.ParseBool(matches[1])
			if err != nil {
				panic(err)
			}

			writeBufferBytes, err = strconv.Atoi(matches[2])
			if err != nil {
				panic(err)
			}

			connectionType = matches[3]
			// fmt.Printf("use_buffer=%t; write_buffer_bytes=%d; connection_type=%s; line=%s\n", useBuffer, writeBufferBytes, connectionType, scanner.Text())
			continue
		}

		matches = resultLine.FindStringSubmatch(scanner.Text())
		if len(matches) > 0 {
			readBufferBytes, err := strconv.Atoi(matches[1])
			if err != nil {
				panic(err)
			}

			duration, err := time.ParseDuration(matches[2])
			if err != nil {
				panic(err)
			}

			numSyscalls, err := strconv.Atoi(matches[3])
			if err != nil {
				panic(err)
			}

			throughputMIBPerSec, err := strconv.ParseFloat(matches[4], 64)
			if err != nil {
				panic(err)
			}

			shortReads, err := strconv.Atoi(matches[5])
			if err != nil {
				panic(err)
			}

			// fmt.Printf("use_buffer=%t; write_buffer_bytes=%d; connection_type=%s; line=%s\n", useBuffer, writeBufferBytes, connectionType, scanner.Text())
			results.Write([]string{machineConfiguration, fmt.Sprint(useBuffer),
				fmt.Sprint(writeBufferBytes), connectionType, fmt.Sprint(readBufferBytes),
				fmt.Sprint(duration.Seconds()), fmt.Sprint(numSyscalls), fmt.Sprint(shortReads),
				fmt.Sprint(throughputMIBPerSec)})
			continue
		}
	}
	if scanner.Err() != nil {
		panic(scanner.Err())
	}
	results.Flush()
}
