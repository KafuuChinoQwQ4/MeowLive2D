import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execute = promisify(execFile);

// Parse every proposed edit before saving it: only the VTS flag may change.
// This also distinguishes real keys from comments and multiline string contents,
// while preserving the user's TOML formatting without another dependency.
const enableVts = String.raw`import copy,os,re,stat,sys,tempfile,tomllib
from pathlib import Path

path=Path(sys.argv[1])
info=path.lstat()
if not stat.S_ISREG(info.st_mode) or info.st_size>65536:
 raise ValueError('invalid configuration file')
original=path.read_bytes()
bom=b'\xef\xbb\xbf' if original.startswith(b'\xef\xbb\xbf') else b''
text=original[len(bom):].decode('utf-8')
data=tomllib.loads(text)
vts=data.get('vtube_studio',{})
if not isinstance(vts,dict) or type(vts.get('enabled',False)) is not bool:
 raise ValueError('invalid VTS enable flag')
if vts.get('enabled',False):
 sys.exit(0)
expected=copy.deepcopy(data)
expected.setdefault('vtube_studio',{})['enabled']=True
newline='\r\n' if '\r\n' in text else '\n'

def candidates():
 if 'enabled' in vts:
  for match in re.finditer(r'\bfalse\b',text):
   yield text[:match.start()]+'true'+text[match.end():]
 elif 'vtube_studio' not in data:
  yield text+('' if not text or text.endswith('\n') else newline)+'[vtube_studio]'+newline+'enabled = true'+newline
 else:
  for match in re.finditer(r'(?m)^\s*\[[^\r\n]+\][^\r\n]*(?:\r?\n|$)',text):
   end=match.end()
   yield text[:end]+('' if text[:end].endswith('\n') else newline)+'enabled = true'+newline+text[end:]
  yield 'vtube_studio.enabled = true'+newline+text
  for match in re.finditer(r'\{',text):
   yield text[:match.end()]+'enabled = true, '+text[match.end():]

updated=None
for candidate in candidates():
 try:
  if tomllib.loads(candidate)==expected:
   updated=bom+candidate.encode('utf-8')
   break
 except tomllib.TOMLDecodeError:
  pass
if updated is None:
 raise ValueError('cannot safely update VTS enable flag')
temporary=None
try:
 with tempfile.NamedTemporaryFile(dir=path.parent,prefix='.'+path.name+'.',delete=False) as output:
  temporary=output.name
  os.fchmod(output.fileno(),stat.S_IMODE(info.st_mode))
  output.write(updated)
  output.flush()
  os.fsync(output.fileno())
 if path.read_bytes()!=original:
  raise ValueError('configuration changed during update')
 os.replace(temporary,path)
 temporary=None
finally:
 if temporary is not None:
  os.unlink(temporary)
`;

export async function enableWindowsVts(configPath) {
  try {
    await execute('python3', ['-c', enableVts, configPath], { timeout: 5000, maxBuffer: 8192 });
  } catch {
    // Parser errors can contain private configuration lines; expose only guidance.
    throw new Error('无法启用 VTube Studio：请检查执行端 TOML 配置有效且可写，并确认已安装 Python 3.11 或更新版本。');
  }
}
