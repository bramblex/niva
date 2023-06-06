#!/usr/bin/env python3
import os
import argparse
import gzip
import json
import os
import struct
import time

# def build_resource(resource_dir, use_gzip):
#     """
#         构建资源索引和资源数据
#     """
#     index = {}
#     data_section = bytearray()
#     offset = 0

#     for root, dirs, files in os.walk(resource_dir):
#         for fname in files:
#             fpath = os.path.join(root, fname)
#             rel_path = os.path.relpath(fpath, resource_dir)
#             with open(fpath, "rb") as f:
#                 data = f.read()
#             original_size = len(data_section)
#             key = "/" + rel_path.replace(os.sep, "/")
#             index[key] = [offset, original_size]
#             data_section.extend(data)
#             offset += original_size

#     index_section = json.dumps(index).encode("utf-8")

#     if use_gzip:
#         data_section = gzip.compress(data_section)

#     header_length = 24
#     index_length = len(index_section)
#     data_offset = header_length + index_length
#     flags = 0x01 if use_gzip else 0x00

#     resource_header = struct.pack("4s B B Q I I",
#                                   b"NIVA",  # Magic Number
#                                   1,           # 版本号
#                                   flags,       # gzip 标志
#                                   int(time.time()),  # 生成时间戳
#                                   index_length,  # JSON 索引区长度
#                                   data_offset,  # 数据区起始位置
#                                   )

#     resource = resource_header + index_section + data_section
#     return resource

# def build_macos_app(config)
# def pack_windows_app(config, app_exe, meta_json, resource_bin):
#     pass
# def sign_windows_app():
#     pass
# def pack_mac_app(config, app_exe, meta_json, resource_bin):
#     pass
# def sign_mac_app():
#     pass
# def create_resource_files(resource_dir, use_gzip):
#     """
#     遍历 resource_dir 下所有文件（包含子目录），将每个文件的数据原样拼接到一起，
#     同时记录每个文件在未压缩数据中的 offset 和原始大小（用于后续解包提取）。
#     如果 use_gzip 为 True，则在所有数据拼接完毕后对整体数据进行 gzip 压缩。
#     """
#     meta = {
#         "gzip": use_gzip,
#         "files": {}
#     }
#     resource_data = bytearray()
#     offset = 0
#     for root, dirs, files in os.walk(resource_dir):
#         for fname in files:
#             fpath = os.path.join(root, fname)
#             rel_path = os.path.relpath(fpath, resource_dir)
#             with open(fpath, "rb") as f:
#                 data = f.read()
#             original_size = len(data)
#             key = "/" + rel_path.replace(os.sep, "/")
#             meta["files"][rel_path] = [offset, original_size]
#             resource_data.extend(data)
#             offset += original_size

#     if use_gzip:
#         resource_data = gzip.compress(resource_data)
#     return resource_data, meta
# def write_resource_files(resource_data, meta):
#     """
#     可选：将生成的 RESOURCE_DATA.bin 和 RESOURCE_META.json 写到磁盘，
#     便于调试或者后续单独使用。
#     """
#     with open("RESOURCE_DATA.bin", "wb") as f:
#         f.write(resource_data)
#     with open("RESOURCE_META.json", "w", encoding="utf-8") as f:
#         json.dump(meta, f, indent=4)


# def update_exe_resources(input_exe, resource_data, meta, output_exe):
#     """
#     采用 Windows API 更新资源的方式将 RESOURCE_DATA 和 RESOURCE_META 嵌入到 exe 中，
#     资源类型使用 RT_RCDATA，资源名称分别为 "RESOURCE_DATA" 和 "RESOURCE_META"。

#     操作流程：
#       1. 将输入 exe 拷贝到输出 exe（因为资源更新操作是就地修改）。
#       2. 调用 BeginUpdateResource 打开输出 exe 进行资源更新。
#       3. 分别调用 UpdateResource 添加两个资源。
#       4. 调用 EndUpdateResource 提交更新。
#     """
#     # 复制输入 exe 到输出 exe
#     shutil.copy2(input_exe, output_exe)
#     # 将 meta 信息转换为 JSON 字符串后编码为字节数据
#     meta_bytes = json.dumps(meta, indent=4).encode("utf-8")
#     # 开始资源更新，第二个参数 False 表示不删除原有的资源
#     hUpdate = win32api.BeginUpdateResource(output_exe, False)
#     try:
#         # RT_RCDATA 对应的常量为 win32con.RT_RCDATA
#         win32api.UpdateResource(
#             hUpdate, win32con.RT_RCDATA, "RESOURCE_DATA", bytes(resource_data))
#         win32api.UpdateResource(
#             hUpdate, win32con.RT_RCDATA, "RESOURCE_META", meta_bytes)
#     except Exception as e:
#         # 出现异常时放弃修改，恢复 exe 原状态
#         win32api.EndUpdateResource(hUpdate, True)
#         raise e
#     # 提交资源更新
#     win32api.EndUpdateResource(hUpdate, False)

from ..resource import build as build_resource

def main():
    parser = argparse.ArgumentParser(
        description="将资源目录打包为 Windows 资源，并嵌入到可执行文件中")
    parser.add_argument("--gzip", action="store_true",
                        help="对整个资源数据使用 gzip 压缩")
    parser.add_argument("resource_dir", help="资源目录路径")
    # parser.add_argument("input_exe", help="待打包的 Windows 可执行文件")
    # parser.add_argument("output_exe", help="打包后的 Windows 可执行文件")
    args = parser.parse_args()

    resource_dir = os.path.abspath(args.resource_dir)
    print("resource_dir: ", resource_dir)
    resource = build_resource(resource_dir, args.gzip)
    with open("resource.bin", "wb") as f:
        f.write(resource)
    print("打包完成，生成的资源文件：", "resource.bin")

    # # 生成资源数据和 meta 信息
    # resource_data, meta = create_resource_files(args.resource_dir, args.gzip)
    # write_resource_files(resource_data, meta)
    # # 更新 exe 内部的资源表，将数据以 RT_RCDATA 资源嵌入进去
    # input_exe = os.path.abspath(args.input_exe)
    # output_exe = os.path.abspath(args.output_exe)
    # update_exe_resources(input_exe, resource_data, meta, output_exe)
    # print("打包完成，生成的可执行文件：", args.output_exe)

if __name__ == "__main__":
    main()
