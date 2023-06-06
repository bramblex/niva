
import os
import argparse
import gzip
import json
import os
import struct
import time


def build(resource_dir, use_gzip):
    """
        构建资源索引和资源数据
    """
    index = {}
    data_section = bytearray()
    offset = 0

    for root, dirs, files in os.walk(resource_dir):
        for fname in files:
            fpath = os.path.join(root, fname)
            rel_path = os.path.relpath(fpath, resource_dir)
            with open(fpath, "rb") as f:
                data = f.read()
            original_size = len(data_section)
            key = "/" + rel_path.replace(os.sep, "/")
            index[key] = [offset, original_size]
            data_section.extend(data)
            offset += original_size

    index_section = json.dumps(index).encode("utf-8")

    if use_gzip:
        data_section = gzip.compress(data_section)

    header_length = 24
    index_length = len(index_section)
    data_offset = header_length + index_length
    flags = 0x01 if use_gzip else 0x00

    resource_header = struct.pack("4s B B Q I I",
                                  b"NIVA",  # Magic Number
                                  1,           # 版本号
                                  flags,       # gzip 标志
                                  int(time.time()),  # 生成时间戳
                                  index_length,  # JSON 索引区长度
                                  data_offset,  # 数据区起始位置
                                  )

    resource = resource_header + index_section + data_section
    return resource
