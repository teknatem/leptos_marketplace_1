-- Add available_models column to a017_llm_agent
ALTER TABLE a017_llm_agent ADD COLUMN available_models TEXT;

-- Create index for better query performance
CREATE INDEX IF NOT EXISTS idx_a017_llm_agent_available_models 
ON a017_llm_agent(available_models) 
WHERE available_models IS NOT NULL;
