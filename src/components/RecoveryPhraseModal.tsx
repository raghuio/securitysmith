import { useState, useEffect, useRef } from "react";
import {
  Button,
  Modal,
  Stack,
  Text,
  TextInput,
  Alert,
  Checkbox,
} from "@mantine/core";
import { validateRecoveryWords } from "../api/auth";
import type { RecoveryInfo, ValidationResult } from "../api/auth";

interface Props {
  opened: boolean;
  recovery: RecoveryInfo;
  onSuccess: () => void;
  onClose: () => void;
  allowClose?: boolean;
}

export function RecoveryPhraseModal({
  opened,
  recovery,
  onSuccess,
  onClose,
  allowClose,
}: Props) {
  const [step, setStep] = useState<"display" | "validate" | "success">(
    "display",
  );
  const [currentRecovery, setCurrentRecovery] =
    useState<RecoveryInfo>(recovery);
  const [words, setWords] = useState(["", "", ""]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [writtenDown, setWrittenDown] = useState(false);
  const inputRefs = useRef<HTMLInputElement[]>([]);

  useEffect(() => {
    setCurrentRecovery(recovery);
    setStep("display");
    setWords(["", "", ""]);
    setError(null);
    setLoading(false);
    setWrittenDown(false);
  }, [recovery.phrase, recovery.positions, recovery.is_rotation]);

  const handleProceed = () => {
    setStep("validate");
    setWords(["", "", ""]);
    setError(null);
  };

  const handleSubmit = async () => {
    setError(null);
    setLoading(true);
    try {
      const result: ValidationResult = await validateRecoveryWords(
        currentRecovery.phrase,
        currentRecovery.positions,
        words,
      );
      if (result.success) {
        setStep("success");
      } else if (result.new_phrase && result.new_positions) {
        setCurrentRecovery({
          phrase: result.new_phrase,
          positions: result.new_positions,
          is_rotation: currentRecovery.is_rotation,
        });
        setStep("display");
        setWords(["", "", ""]);
        setWrittenDown(false);
        setError(
          "One or more words were incorrect. A new recovery phrase has been generated. Please write it down.",
        );
      } else {
        setError("Validation failed.");
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      opened={opened}
      onClose={allowClose ? onClose : () => {}}
      title={
        currentRecovery.is_rotation ? "New Recovery Phrase" : "Recovery Phrase"
      }
      closeOnClickOutside={allowClose ?? false}
      closeOnEscape={allowClose ?? false}
      withCloseButton={allowClose ?? false}
      centered
    >
      <Stack>
        {error && (
          <Alert color="red" variant="light">
            {error}
          </Alert>
        )}

        {step === "display" && (
          <>
            <Text c="red" fw={700}>
              Write this down. You will never see it again.
            </Text>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(3, 1fr)",
                gap: "8px 16px",
                fontFamily: "monospace",
                fontSize: "1.1rem",
                fontWeight: 600,
                userSelect: "none",
                lineHeight: 1.5,
              }}
            >
              {currentRecovery.phrase.split(" ").map((word, i) => (
                <div key={i}>
                  <span style={{ color: "#868e96", marginRight: 6 }}>
                    {i + 1}.
                  </span>
                  {word}
                </div>
              ))}
            </div>
            <Checkbox
              label="I have written down this recovery phrase"
              checked={writtenDown}
              onChange={(e) => setWrittenDown(e.currentTarget.checked)}
            />
            <Button onClick={handleProceed} disabled={!writtenDown}>
              Proceed to Validation
            </Button>
          </>
        )}

        {step === "validate" && (
          <>
            <Text fw={600} size="sm">
              To confirm you have recorded it, please enter the following words:
            </Text>
            {currentRecovery.positions.map((pos, idx) => (
              <TextInput
                key={`${currentRecovery.phrase}-${pos}`}
                label={`Word #${pos + 1}`}
                placeholder={`Enter word ${pos + 1}...`}
                value={words[idx]}
                autoFocus={idx === 0}
                onChange={(e) => {
                  const next = [...words];
                  next[idx] = e.currentTarget.value;
                  setWords(next);
                }}
                onKeyDown={(e) => {
                  if (e.key !== "Enter") return;
                  e.preventDefault();
                  if (idx < currentRecovery.positions.length - 1) {
                    inputRefs.current[idx + 1]?.focus();
                  } else {
                    handleSubmit();
                  }
                }}
                ref={(el) => {
                  if (el) inputRefs.current[idx] = el;
                }}
              />
            ))}
            <Button onClick={handleSubmit} loading={loading}>
              Validate
            </Button>
          </>
        )}

        {step === "success" && (
          <>
            <Alert color="green" variant="light" title="Recovery Phrase Saved">
              Your recovery phrase has been securely encrypted and stored. You
              can now use it to recover your vault if you forget your master
              password.
            </Alert>
            <Button onClick={onSuccess}>Continue</Button>
          </>
        )}
      </Stack>
    </Modal>
  );
}
