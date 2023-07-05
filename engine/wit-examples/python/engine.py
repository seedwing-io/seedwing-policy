from dist import Root, RootImports
from wasmtime import Store
from Crypto.Random import get_random_bytes
from dist.imports.types import (RuntimeValueObject,
                                Object,
                                ObjectValueBoolean,
                                ObjectValueString,
                                Pattern,
                                Rationale)
from typing import Mapping, Tuple, List, Tuple

from dist.types import Result
from dist.types import Ok

from dist.imports import Random
from dist.imports import Stdin
from dist.imports import Stdout
from dist.imports import Stderr
from dist.imports import streams
from dist.imports import Preopens
from dist.imports import Environment
from dist.imports import filesystem
from dist.imports import Filesystem
from dist.imports.filesystem import Descriptor, Filesize, ErrorCode, DescriptorType
import sys
import pprint

class WasiRandom(Random):
    def get_random_bytes(self, len: int) -> bytes:
        return get_random_bytes(len)

class WasiStdin(Stdin):
    def get_stdin(self) -> streams.InputStream:
        return sys.stdin.fileno()

class WasiStdout(Stdout):
    def get_stdout(self) -> streams.OutputStream:
        return sys.stdout.fileno()

class WasiStderr(Stderr):
    def get_stderr(self) -> streams.OutputStream:
        return sys.stderr.fileno()

class WasiPreopens(Preopens):
    def get_directories(self) -> List[Tuple[filesystem.Descriptor, str]]:
        return []

class WasiStreams(streams.Streams):
    def drop_input_stream(self, this: streams.InputStream) -> None:
        return None

    def write(self, this: streams.OutputStream, buf: bytes) -> Result[int, streams.StreamError]:
        sys.stdout.buffer.write(buf)
        return Ok(len(buf))

    def blocking_write(self, this: streams.OutputStream, buf: bytes) -> Result[int, streams.StreamError]:
        sys.stdout.buffer.write(buf)
        return Ok(len(buf))

    def drop_output_stream(self, this: streams.OutputStream) -> None:
        return None

class WasiEnvironment(Environment):
    def get_environment(self) -> List[Tuple[str, str]]:
        return []

class WasiFilesystem(Filesystem):
    def write_via_stream(self, this: Descriptor, offset: Filesize) -> Result[streams.OutputStream, ErrorCode]:
        raise NotImplementedError
    def append_via_stream(self, this: Descriptor) -> Result[streams.OutputStream, ErrorCode]:
        raise NotImplementedError
    def get_type(self, this: Descriptor) -> Result[DescriptorType, ErrorCode]:
        raise NotImplementedError
    def drop_descriptor(self, this: Descriptor) -> None:
        raise NotImplementedError

def main():
    store = Store()
    root = Root(store, RootImports(None,
                         WasiStreams(),
                         WasiFilesystem,
                         WasiRandom(),
                         WasiEnvironment(),
                         WasiPreopens(),
                         None,
                         WasiStdin(),
                         WasiStdout(),
                         WasiStderr()))

    engine = root.engine()

    print(f'Seedwing Policy Engine version: {engine.version(store)}')

    policies = []
    data = []
    policy = "pattern dog = { name: string, trained: boolean }"
    policy_name = "dog"
    input = RuntimeValueObject([
        Object("name", ObjectValueString("goodboy")),
        Object("trained", ObjectValueBoolean(True))])

    result: EvaluationResultContext = engine.eval(store, policies, data, policy, policy_name, input)

    if result.value:
        print('EvaluationResult:')

        input: RuntimeValue = result.value.evaluation_result.input
        print('input: ', end='')
        pprint.pprint(input)

        ty: Pattern = result.value.evaluation_result.ty;
        print('ty: ', end='')
        pprint.pprint(ty)

        rationale: Rationale = result.value.evaluation_result.rationale
        print('rationale: ', end='')
        pprint.pprint(rationale)

        output: String = result.value.evaluation_result.output
        print('output: ', end='')
        pprint.pprint(output)

        #map = result.value.pattern_map
        #print('pattern_map: ', end='')
        #pprint.pprint(map)
    else:
        print(result)

if __name__ == '__main__':
    main()
