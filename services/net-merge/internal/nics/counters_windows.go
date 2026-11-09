// Package nics - Windows per-interface byte counters (GetIfEntry2Ex).
//
// Vendored from net-merge (commit 2b333a3ad820996ba337588adfa8f7259a45baec,
// Release 0.3.7), MIT licensed (Copyright 2026 Rohit Mahto).
package nics

import "golang.org/x/sys/windows"

// Octets reads Windows' own byte counters for an adapter. They cover every app's traffic, not
// just what net-merge relays: apps like qBittorrent bind to each adapter directly and bypass the
// TUN, but still show up here.
func Octets(idx int) (in, out uint64, ok bool) {
	if idx <= 0 {
		return 0, 0, false
	}
	row := windows.MibIfRow2{InterfaceIndex: uint32(idx)}
	if windows.GetIfEntry2Ex(windows.MibIfEntryNormal, &row) != nil {
		return 0, 0, false
	}
	return row.InOctets, row.OutOctets, true
}
