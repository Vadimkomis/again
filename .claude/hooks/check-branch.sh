#!/bin/bash

# Deny file edits on main branch

current_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)

if [ "$current_branch" = "main" ] || [ "$current_branch" = "master" ]; then
  echo "Edits blocked: currently on protected branch '$current_branch'" >&2
  exit 2  # Exit code 2 = deny permission
fi

exit 0  # Allow edit
