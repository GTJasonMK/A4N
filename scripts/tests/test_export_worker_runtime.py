import unittest

from run_export_worker_runtime_e2e import self_test


class ExportWorkerRuntimeTest(unittest.TestCase):
    def test_runtime_contracts_and_resource_ownership(self):
        self_test()
