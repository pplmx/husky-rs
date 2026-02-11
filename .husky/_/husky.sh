#!/bin/sh
if [ -z "$husky_skip_init" ]; then
  if [ "$HUSKY" = "0" ]; then
    exit 0
  fi
  readonly husky_skip_init=1
fi
