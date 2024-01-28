# Super easy way to get some basic CI/CD going

import sys
import subprocess
import increment_version


def main():
    if len(sys.argv) != 2:
        print("Usage: python3 commit.py <commit message>")
        sys.exit(1)

    commit_message = sys.argv[1]

    try:
        subprocess.check_call(["cargo", "build", "--release"])
    except subprocess.CalledProcessError:
        print("Failed to build")
        sys.exit(1)

    try:
        subprocess.check_call(["cargo", "fmt"])
    except subprocess.CalledProcessError:
        print("Failed to format")
        sys.exit(1)

    try:
        subprocess.check_call(["cargo", "clippy"])
    except subprocess.CalledProcessError:
        print("Failed to clippy")
        sys.exit(1)

    increment_version.increment_version()    

    try:
        subprocess.check_call(["git", "add", "."])
        subprocess.check_call(["git", "commit", "-m", commit_message])
    except subprocess.CalledProcessError:
        print("Failed to commit")
        sys.exit(1)

    try:
        subprocess.check_call(["git", "push"])
    except subprocess.CalledProcessError:
        print("Failed to push")
        sys.exit(1)

    print("Successfully pushed")

if __name__ == "__main__":
    main()