import platform
import requests
import os
import tarfile
import tempfile

LINUX_RELEASE_URL = "https://github.com/newtoallofthis123/ynk/releases/download/v.0.1.9/ynk_v.0.1.9.tar.xz"
DESTINATION = "ynk_v.0.1.9.tar.xz"


def get_release_url():
    system = platform.system()
    if system == "Windows":
        print("Windows is not supported yet")
        print("Use cargo install ynk instead")
        exit(0)
    elif system == "Darwin":
        print("Mac is not supported yet")
        print("Use cargo install ynk instead")
        exit(0)
    elif system == "Linux":
        return LINUX_RELEASE_URL
    else:
        print("Unknown system")
        exit(0)


temp_dir = tempfile.gettempdir()
tar_file_path = os.path.join(temp_dir, DESTINATION)
des_dir = os.path.expanduser("/usr/local/bin/")
url = get_release_url()
response = requests.get(url)

with open(tar_file_path, "wb") as file:
    file.write(response.content)


with tarfile.open(tar_file_path, "r:xz") as tar:
    tar.extractall(temp_dir)

print("Give me the sudo power!!")
os.system(f"sudo mv {temp_dir}/ynk {des_dir}")

print(f"File downloaded to {DESTINATION}")
