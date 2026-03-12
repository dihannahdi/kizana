import zipfile, shutil, os, sys
src = '/opt/kizana/backend/src'
bk  = '/opt/kizana/backend/src_v26_backup'
zippath = '/tmp/backend_src_v27.zip'

# Backup current src
if not os.path.exists(bk):
    shutil.copytree(src, bk)
    print(f"Backed up src to {bk}")
else:
    print(f"Backup at {bk} already exists, skipping")

# Extract zip 
with zipfile.ZipFile(zippath) as z:
    z.extractall('/opt/kizana/backend/')
print('Extracted OK')
ls = os.listdir('/opt/kizana/backend/src')
print(f'{len(ls)} files in src/')
