# Used for systematically incrementing the version number in Cargo.toml

def increment_version():
    with open("Cargo.toml", "r") as file:
        data = file.readlines()

    version_line = None

    for i in range(len(data)):
        if data[i].startswith("version"):
            version_line = i
            break

    version = data[version_line].split("=")[1].strip().replace('"', "")
    version = version.split(".")
    version[-1] = str(int(version[-1]) + 1)

    if version[-1] == "10":
        version[-1] = "0"
        version[-2] = str(int(version[-2]) + 1)

    version = ".".join(version)

    data[version_line] = f'version = "{version}"\n'

    with open("Cargo.toml", "w") as file:
        file.writelines(data)

if __name__ == "__main__":
    increment_version()