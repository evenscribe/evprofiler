# EvProfiler

EvProfiler is a continuous profiling system that collects, processes, and analyzes performance data using parca-agent for raw data collection. This project aims to provide a robust profiling solution with efficient storage and query capabilities.

## Overview

EvProfiler builds upon the foundations of [parca-agent](https://github.com/parca-dev/parca-agent) for data collection while implementing its own server-side components for processing, storage, and analysis. The system is designed to handle continuous profiling data efficiently with a focus on debuggability and performance.

## Features

### Implemented

- **Profile Store**: Core storage system for managing profiling data
- **Debug Info Store**: 
  - Integrated debug information management
  - Automated debug info downloads via DebugInfod
- **Advanced Symbolization**:
  - DWARF walking implementation
  - SymTab/DynSym support
  - Comprehensive symbol resolution capabilities
- **Efficient Storage**:
  - Parquet-based storage format
  - Optimized for query performance and storage efficiency

### In Progress

- **Column Query System**:
  - Currently implementing FlatPprofGenerator
  - Designed for efficient profile data querying

### Planned Features

- **Remote Storage Support**:
  - Current implementation is filesystem-based
  - Abstracted through object_store interface for easy extension
  - Future support for various remote storage backends
- **HTTP Query Proxy**:
  - REST API for ColumnQuery operations
  - Enhanced data access capabilities

## Architecture

EvProfiler follows a modular architecture with the following key components:

1. **Data Collection**: 
   - Utilizes parca-agent for raw profiling data collection
   - Compatible with parca's data formats and collection mechanisms

2. **Storage Layer**:
   - Parquet-based storage for efficient data management
   - Abstracted object store interface for storage flexibility

3. **Debug Information Management**:
   - Integrated debug symbol handling
   - Automated debug info retrieval
   - Comprehensive symbolization support

4. **Query System**:
   - Column-oriented query capabilities
   - Optimized for profile data analysis

## Acknowledgments

- [Parca](https://github.com/parca-dev) project for providing the agent and server reference implementation
