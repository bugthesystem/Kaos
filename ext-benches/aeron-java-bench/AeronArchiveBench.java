package com.kaos;

import io.aeron.Aeron;
import io.aeron.Publication;
import io.aeron.archive.Archive;
import io.aeron.archive.ArchiveThreadingMode;
import io.aeron.archive.client.AeronArchive;
import io.aeron.archive.codecs.SourceLocation;
import io.aeron.driver.MediaDriver;
import io.aeron.driver.ThreadingMode;
import org.agrona.concurrent.UnsafeBuffer;

import java.io.File;
import java.nio.file.Files;

/**
 * Aeron Archive Benchmark - measures archive throughput
 *
 * Run with:
 * mvn compile exec:java -Dexec.mainClass="com.kaos.AeronArchiveBench"
 */
public class AeronArchiveBench {
    private static final int MESSAGE_COUNT = 1_000_000;
    private static final int MESSAGE_SIZE = 64;
    private static final String CHANNEL = "aeron:ipc";
    private static final int STREAM_ID = 1001;

    public static void main(String[] args) throws Exception {
        System.out.println("=== Aeron Archive Benchmark ===");
        System.out.println("Messages: " + MESSAGE_COUNT);
        System.out.println("Message size: " + MESSAGE_SIZE + " bytes");
        System.out.println();

        // Create temp directories
        File aeronDir = Files.createTempDirectory("aeron-bench").toFile();
        File archiveDir = Files.createTempDirectory("aeron-archive").toFile();

        aeronDir.deleteOnExit();
        archiveDir.deleteOnExit();

        // Start Media Driver with low-latency settings
        MediaDriver.Context driverCtx = new MediaDriver.Context()
            .aeronDirectoryName(aeronDir.getAbsolutePath())
            .threadingMode(ThreadingMode.SHARED)
            .dirDeleteOnStart(true)
            .dirDeleteOnShutdown(true);

        // Start Archive
        Archive.Context archiveCtx = new Archive.Context()
            .aeronDirectoryName(aeronDir.getAbsolutePath())
            .archiveDir(archiveDir)
            .threadingMode(ArchiveThreadingMode.SHARED)
            .deleteArchiveOnStart(true);

        try (MediaDriver driver = MediaDriver.launch(driverCtx);
             Archive archive = Archive.launch(archiveCtx)) {

            // Connect to Archive
            AeronArchive.Context archiveClientCtx = new AeronArchive.Context()
                .aeronDirectoryName(aeronDir.getAbsolutePath());

            try (AeronArchive aeronArchive = AeronArchive.connect(archiveClientCtx)) {

                // Start recording
                aeronArchive.startRecording(CHANNEL, STREAM_ID, SourceLocation.LOCAL);

                // Get a publication
                try (Publication publication = aeronArchive.context().aeron()
                        .addPublication(CHANNEL, STREAM_ID)) {

                    // Wait for connection
                    while (!publication.isConnected()) {
                        Thread.yield();
                    }

                    // Prepare message
                    UnsafeBuffer buffer = new UnsafeBuffer(new byte[MESSAGE_SIZE]);
                    for (int i = 0; i < MESSAGE_SIZE; i++) {
                        buffer.putByte(i, (byte) 0);
                    }

                    // Warmup
                    System.out.println("Warming up...");
                    for (int i = 0; i < 100_000; i++) {
                        while (publication.offer(buffer, 0, MESSAGE_SIZE) < 0) {
                            Thread.yield();
                        }
                    }
                    Thread.sleep(500);

                    // Benchmark
                    System.out.println("Benchmarking " + MESSAGE_COUNT + " messages...");
                    long start = System.nanoTime();
                    long backpressure = 0;

                    for (int i = 0; i < MESSAGE_COUNT; i++) {
                        while (publication.offer(buffer, 0, MESSAGE_SIZE) < 0) {
                            backpressure++;
                            Thread.yield();
                        }
                    }

                    long sendTime = System.nanoTime() - start;

                    // Wait for archive to catch up
                    Thread.sleep(1000);
                    long totalTime = System.nanoTime() - start;

                    // Results
                    double sendSec = sendTime / 1_000_000_000.0;
                    double totalSec = totalTime / 1_000_000_000.0;
                    double sendRate = MESSAGE_COUNT / sendSec / 1_000_000.0;
                    double totalRate = MESSAGE_COUNT / totalSec / 1_000_000.0;
                    double bandwidth = (MESSAGE_COUNT * (long) MESSAGE_SIZE) / totalSec / (1024 * 1024 * 1024.0);

                    System.out.println();
                    System.out.println("=== Results ===");
                    System.out.printf("Send time:       %.3f ms%n", sendTime / 1_000_000.0);
                    System.out.printf("Total time:      %.3f ms%n", totalTime / 1_000_000.0);
                    System.out.printf("Send rate:       %.2f M/s%n", sendRate);
                    System.out.printf("Total rate:      %.2f M/s%n", totalRate);
                    System.out.printf("Bandwidth:       %.2f GB/s%n", bandwidth);
                    System.out.printf("Backpressure:    %d%n", backpressure);
                    System.out.printf("Latency/msg:     %.1f ns%n", (double) sendTime / MESSAGE_COUNT);
                }

                // Stop recording
                aeronArchive.stopRecording(CHANNEL, STREAM_ID);
            }
        }

        System.out.println("\nDone.");
    }
}
