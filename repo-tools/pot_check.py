# SPDX-License-Identifier: CC0-1.0

# CI check to make sure POTFILES.in is up to date.
# If this fails, run pot_update.py

import os
import filecmp
from pot_update import update_potfiles, PO_DIR

if __name__ == "__main__":
    update_potfiles("POTFILES.in.temp")
    os.chdir(PO_DIR)
    if not filecmp.cmp("POTFILES.in", "POTFILES.in.temp", shallow=False):
        print("potfile doesn't match")

        with open("POTFILES.in.temp", "r") as f:
            print(f"\nexpected:\n{f.read()}")

        with open("POTFILES.in", "r") as f:
            print(f"\ngot:\n{f.read()}")

        exit(1)

    print("Success")
