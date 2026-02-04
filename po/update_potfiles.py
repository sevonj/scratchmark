# Auto crawls every ui file into POTFILES.in

import os

PO_DIR = os.path.dirname(os.path.realpath(__file__))
ROOT_DIR = os.path.dirname(PO_DIR)
IN_IN_PATH = os.path.join(PO_DIR, "POTFILES.in.in")

def update_potfiles(filename: str = "POTFILES.in"):
    os.chdir(ROOT_DIR)

    in_in: str
    with open(IN_IN_PATH, "r") as f:
        in_in = f.read()

    with open(os.path.join(PO_DIR, filename), "w") as f:
        f.write(in_in)
        potfiles = []
        for root, _, files in os.walk("data/resources"):
            for file in files:
                if file.endswith(".ui"):
                    potfiles.append(os.path.join(root, file) + "\n")
        potfiles.sort()
        f.writelines(potfiles)


if __name__ == "__main__":
    update_potfiles()
