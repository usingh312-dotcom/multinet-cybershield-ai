// Package logx provides simple leveled logging for the Net Merge service.
//
// Vendored from net-merge (commit 2b333a3ad820996ba337588adfa8f7259a45baec,
// Release 0.3.7), MIT licensed (Copyright 2026 Rohit Mahto).
package logx

import (
	"io"
	"log"
	"os"
)

var std = log.New(os.Stderr, "", log.LstdFlags)

// SetOutput redirects log output to w.
func SetOutput(w io.Writer) { std.SetOutput(w) }

// Printf logs a formatted message at the default level.
func Printf(format string, v ...any) { std.Printf(format, v...) }

// Println logs a message at the default level.
func Println(v ...any) { std.Println(v...) }
