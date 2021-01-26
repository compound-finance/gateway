#!/usr/bin/env python

import sys
import json

for line in sys.stdin:
    message = json.loads(line)
    if message.get('executable'):
        print(message['executable'])
