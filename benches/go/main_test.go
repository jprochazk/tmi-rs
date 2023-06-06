package main

import (
	"bufio"
	"fmt"
	"os"
	"testing"

	"github.com/Mm2PL/justgrep"
	"github.com/jprochazk/go-twitch-irc/v4"
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

	b.Run("justgrep", func(b *testing.B) {
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

	b.Run("go-twitch-irc", func(b *testing.B) {
		for i := 0; i < b.N; i++ {
			for _, line := range input {
				message, err := twitch.ParseIRCMessage(line)
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
