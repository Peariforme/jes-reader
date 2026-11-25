#!/bin/sh
# Git "clean" filter: replaces IBM Z Xplore user IDs (Z followed by
# exactly 5 digits) with Z99999 in corpus files on `git add`. The
# working copy keeps the real values.
#
# The replacement has the same length, so sysout columns do not move.
# Reads and writes in binary mode, so CRLF line endings are kept (the
# sed shipped with Git for Windows drops them, hence perl).
#
# Reads stdin, writes stdout. Setup: see the README, Development section.
exec perl -0777 -pe 'BEGIN { binmode STDIN; binmode STDOUT } s/(?<![A-Za-z0-9])Z[0-9]{5}(?![0-9])/Z99999/g'
