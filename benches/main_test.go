package main

import (
	"bufio"
	"fmt"
	"os"
	"testing"

	"github.com/Mm2PL/justgrep"
)

func readInput() ([]string, error) {
	file, err := os.Open("data.txt")
	if err != nil {
		return nil, err
	}
	scan := bufio.NewScanner(file)
	scan.Split(bufio.ScanLines)

	lines := []string{}

	n := 1000
	for n != 0 && scan.Scan() {
		lines = append(lines, scan.Text())
		n--
	}

	err = scan.Err()
	if err != nil {
		return nil, err
	}

	return lines, nil
}

func BenchmarkParse(b *testing.B) {
	input, err := readInput()
	if err != nil {
		fmt.Println(err)
		b.FailNow()
		return
	}

	b.Run("justgrep 1000 lines from data.txt", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			for _, line := range input {
				message, err := justgrep.NewMessage(line)
				if err != nil {
					fmt.Println(err)
					b.FailNow()
					return
				}
				_ = message
			}
		}
	})
}
