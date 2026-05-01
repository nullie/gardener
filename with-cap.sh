#!/usr/bin/env bash

sudo -E capsh --caps="cap_setpcap,cap_setuid,cap_setgid+ep cap_dac_read_search+eip" --keep=1 --user="$USER" --addamb="cap_dac_read_search" --shell=/usr/bin/env -- "$@"
